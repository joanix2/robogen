#!/usr/bin/env python3
"""Check production crate boundaries without compiling or downloading dependencies."""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path

CORE_DEPENDENCIES = {
    "robogen-domain": set(),
    "robogen-dsl": {"robogen-domain"},
    "robogen-sketch": {"robogen-domain"},
    "robogen-ir": {"robogen-domain", "robogen-dsl", "robogen-sketch"},
    "robogen-assembly": {"robogen-domain"},
    "robogen-robotics": {"robogen-domain"},
}
FRAMEWORKS = {"egui", "eframe", "wgpu", "rapier3d", "csgrs", "faer", "burn"}
ADAPTERS = {
    "robogen-cad",
    "robogen-constraints",
    "robogen-topopt",
    "robogen-physics",
    "robogen-render",
    "robogen-export",
    "robogen-rl",
}


def production_dependencies(manifest: Path) -> tuple[str, set[str]]:
    """Include aliases, build dependencies and target-specific production edges."""
    data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    name: str = data["package"]["name"]
    tables = [data]
    tables.extend(data.get("target", {}).values())
    dependencies: set[str] = set()
    for table in tables:
        for section in ("dependencies", "build-dependencies"):
            for alias, specification in table.get(section, {}).items():
                package = (
                    specification.get("package", alias)
                    if isinstance(specification, dict)
                    else alias
                )
                dependencies.add(package)
    return name, dependencies


def workspace_dependencies(root: Path) -> dict[str, set[str]]:
    """Read the workspace members declared by Cargo, excluding dev dependencies."""
    workspace = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    manifests: set[Path] = set()
    for pattern in workspace["workspace"]["members"]:
        manifests.update(path / "Cargo.toml" for path in root.glob(pattern))
    if "package" in workspace:
        manifests.add(root / "Cargo.toml")
    excluded = {
        path / "Cargo.toml"
        for pattern in workspace["workspace"].get("exclude", [])
        for path in root.glob(pattern)
    }
    graph: dict[str, set[str]] = {}
    for manifest in sorted(manifests - excluded):
        if not manifest.is_file():
            continue
        name, dependencies = production_dependencies(manifest)
        if name in graph:
            raise ValueError(f"Duplicate workspace package: {name}")
        graph[name] = dependencies
    return graph


def boundary_errors(graph: dict[str, set[str]]) -> list[str]:
    """Enforce the ownership rules recorded in ADR-013 and AGENTS.md."""
    errors: list[str] = []
    for owner, dependencies in sorted(graph.items()):
        internal = {name for name in dependencies if name in graph}
        if owner in CORE_DEPENDENCIES:
            forbidden = internal - CORE_DEPENDENCIES[owner]
            forbidden |= dependencies & FRAMEWORKS
        elif owner in ADAPTERS:
            forbidden = internal & {"robogen-project", "robogen-ui"}
            forbidden |= {
                name
                for name in internal
                if name.startswith("robogen-")
                and name in {"robogen-cli", "robogen-desktop", "robogen-server"}
            }
        else:
            forbidden = set()
        if owner == "robogen-render":
            forbidden |= dependencies & {
                "egui",
                "eframe",
                "robogen-cad",
                "robogen-physics",
                "robogen-topopt",
            }
        if owner == "robogen-ui":
            forbidden |= dependencies & {
                "robogen-constraints",
                "robogen-topopt",
                "robogen-physics",
                "robogen-rl",
            }
        errors.extend(
            f"Forbidden production dependency: {owner} -> {name}"
            for name in sorted(forbidden)
        )

    visited: set[str] = set()
    active: list[str] = []

    def visit(name: str) -> None:
        if name in active:
            cycle = active[active.index(name) :] + [name]
            errors.append("Production dependency cycle: " + " -> ".join(cycle))
            return
        if name in visited:
            return
        active.append(name)
        for dependency in sorted(graph[name] & graph.keys()):
            visit(dependency)
        active.pop()
        visited.add(name)

    for name in sorted(graph):
        visit(name)
    return errors


def main() -> int:
    """Report boundary violations using a nonzero process status."""
    root = Path(__file__).resolve().parents[1]
    try:
        graph = workspace_dependencies(root)
        errors = boundary_errors(graph)
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Architecture check failed: {error}", file=sys.stderr)
        return 1
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    edges = sum(len(dependencies & graph.keys()) for dependencies in graph.values())
    print(
        f"Architecture: {len(graph)} crates, {edges} production edges, boundaries valid"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
