use super::*;

impl RoboGenUi {
    pub fn load_servo_example(&mut self) {
        self.set_dsl_source(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/servo/main.rgn"
        )));
        self.camera = Camera {
            target: robogen_render::Vec3::new(0.0, 1.3, 0.0),
            distance: 6.5,
            ..Camera::default()
        };
        self.design_tab = DesignTab::Viewport;
        self.mode = WorkspaceMode::Design;
    }

    pub fn load_compute_board_example(&mut self) {
        self.set_dsl_source(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/compute_board/main.rgn"
        )));
        self.camera = Camera {
            target: robogen_render::Vec3::new(0.0, 0.4, 0.0),
            yaw: 0.7,
            pitch: 0.75,
            distance: 4.8,
            ..Camera::default()
        };
        self.design_tab = DesignTab::Viewport;
        self.mode = WorkspaceMode::Design;
    }

    pub fn load_camera_example(&mut self) {
        self.set_dsl_source(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/raspberry_pi_camera/main.rgn"
        )));
        self.camera = Camera {
            target: robogen_render::Vec3::new(0.0, 0.4, 0.0),
            yaw: 0.5,
            pitch: 0.85,
            distance: 4.8,
            ..Camera::default()
        };
        self.design_tab = DesignTab::Viewport;
        self.mode = WorkspaceMode::Design;
    }

    pub fn load_battery_example(&mut self) {
        self.set_dsl_source(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/li_ion_battery/main.rgn"
        )));
        self.camera = Camera {
            target: robogen_render::Vec3::new(0.0, 0.4, 0.0),
            yaw: 0.5,
            pitch: 0.55,
            distance: 4.8,
            ..Camera::default()
        };
        self.design_tab = DesignTab::Viewport;
        self.mode = WorkspaceMode::Design;
    }

    pub fn load_topology_example(&mut self) {
        self.set_dsl_source(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/topology_battery_support/main.rgn"
        )));
        self.design_tab = DesignTab::Viewport;
        self.mode = WorkspaceMode::Design;
        self.camera = Camera::default();
    }

    pub fn load_biped_example(&mut self) {
        self.set_dsl_source(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/mini_biped/main.rgn"
        )));
        self.camera = Camera {
            target: robogen_render::Vec3::new(0.0, 1.5, 0.0),
            yaw: 0.25,
            pitch: 0.15,
            distance: 4.8,
            ..Camera::default()
        };
        self.design_tab = DesignTab::Viewport;
        self.mode = WorkspaceMode::Design;
    }

    pub fn load_kinematic_vise_example(&mut self) {
        self.set_dsl_source(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/kinematic_vise/main.rgn"
        )));
        self.mode = WorkspaceMode::Design;
        self.design_tab = DesignTab::Kinematics;
        self.graph_canvas = Default::default();
        self.camera = Camera::default();
    }

    pub fn load_manual_topology_example(&mut self) {
        self.set_dsl_source(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/topology_manual/main.rgn"
        )));
        self.mode = WorkspaceMode::Optimization;
        self.camera = Camera::default();
    }
}
