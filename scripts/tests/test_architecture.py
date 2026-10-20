"""Regression cases for ownership and cycle detection in the production graph."""

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_architecture import boundary_errors, production_dependencies  # noqa: E402


class ArchitectureContracts(unittest.TestCase):
    def test_domain_cannot_depend_on_backend_or_ui_framework(self) -> None:
        errors = boundary_errors(
            {
                "robogen-domain": {"robogen-cad", "egui"},
                "robogen-cad": set(),
            }
        )
        self.assertEqual(len(errors), 2)
        self.assertTrue(
            any("robogen-domain -> robogen-cad" in error for error in errors)
        )
        self.assertTrue(any("robogen-domain -> egui" in error for error in errors))

    def test_orchestration_can_compose_adapters_but_reverse_edge_is_rejected(
        self,
    ) -> None:
        valid = {"robogen-project": {"robogen-cad"}, "robogen-cad": set()}
        self.assertEqual(boundary_errors(valid), [])
        invalid = {
            "robogen-project": {"robogen-cad"},
            "robogen-cad": {"robogen-project"},
        }
        errors = boundary_errors(invalid)
        self.assertTrue(any("Forbidden" in error for error in errors))
        self.assertTrue(any("cycle" in error for error in errors))

    def test_renderer_stays_independent_from_toolkit_and_kernel(self) -> None:
        errors = boundary_errors(
            {"robogen-render": {"eframe", "robogen-cad"}, "robogen-cad": set()}
        )
        self.assertEqual(len(errors), 2)

    def test_aliases_and_target_dependencies_cannot_hide_frameworks(self) -> None:
        source = """[package]
name = "robogen-domain"
[dependencies]
widgets = { package = "egui", version = "0.31" }
[target.'cfg(unix)'.dependencies]
kernel = { package = "csgrs", version = "0.1" }
[dev-dependencies]
robogen-testkit = { path = "../robogen-testkit" }
"""
        with tempfile.TemporaryDirectory() as directory:
            manifest = Path(directory) / "Cargo.toml"
            manifest.write_text(source)
            owner, dependencies = production_dependencies(manifest)
        self.assertEqual(dependencies, {"egui", "csgrs"})
        self.assertEqual(len(boundary_errors({owner: dependencies})), 2)


if __name__ == "__main__":
    unittest.main()
