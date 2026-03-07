use macroquad::prelude::*;
use clap::Parser;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

// ==========================================
// 1. CONFIGURATION (Fine-Tuning via CLI)
// ==========================================
#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "Procedural Ice Growth Simulator")]
pub struct SimConfig {
    /// Total number of simulation frames
    #[arg(short, long, default_value_t = 150)]
    pub frames: usize,

    /// How many particles to spawn per frame
    #[arg(short, long, default_value_t = 150)]
    pub spawn_rate: usize,

    /// Minimum size of an ice particle
    #[arg(long, default_value_t = 0.02)]
    pub radius_min: f32,

    /// Maximum size of an ice particle
    #[arg(long, default_value_t = 0.06)]
    pub radius_max: f32,

    /// How stiff the ice packing is (higher = harder ice, but slower simulation)
    #[arg(long, default_value_t = 3)]
    pub relaxation_steps: usize,
}

// ==========================================
// 2. GEOMETRY (The Mold Shape)
// ==========================================
pub mod geometry {
    use super::*;

    pub struct Plane {
        pub normal: Vec3,
        pub distance: f32,
    }

    pub struct Mold {
        planes: Vec<Plane>,
    }

    impl Mold {
        pub fn default_crystal() -> Self {
            let mut planes = Vec::new();
            planes.push(Plane { normal: vec3(0.0, 0.0, 1.0).normalize(), distance: 0.6 });
            planes.push(Plane { normal: vec3(0.0, 0.0, -1.0).normalize(), distance: 0.6 });
            planes.push(Plane { normal: vec3(-1.0, 0.0, 0.0).normalize(), distance: 1.1 });
            planes.push(Plane { normal: vec3(1.0, 0.0, 0.0).normalize(), distance: 1.1 });
            planes.push(Plane { normal: vec3(-0.4, 1.0, 0.0).normalize(), distance: 1.6 }); 
            planes.push(Plane { normal: vec3(0.6, 1.0, 0.0).normalize(), distance: 1.8 }); 
            planes.push(Plane { normal: vec3(-1.0, -1.0, 0.0).normalize(), distance: 1.3 }); 
            planes.push(Plane { normal: vec3(0.8, -1.2, 0.0).normalize(), distance: 1.2 }); 
            planes.push(Plane { normal: vec3(-0.8, -0.8, 0.6).normalize(), distance: 1.1 }); 
            planes.push(Plane { normal: vec3(0.8, 0.8, 0.6).normalize(), distance: 1.3 }); 
            planes.push(Plane { normal: vec3(0.6, -0.8, -0.6).normalize(), distance: 1.1 }); 
            planes.push(Plane { normal: vec3(-0.6, 0.8, -0.6).normalize(), distance: 1.3 }); 
            Self { planes }
        }

        pub fn constrain(&self, pos: &mut Vec3, radius: f32) {
            for _ in 0..3 {
                for plane in &self.planes {
                    let dist = pos.dot(plane.normal) - plane.distance;
                    if dist > -radius {
                        let penetration = dist + radius;
                        *pos -= plane.normal * penetration;
                    }
                }
            }
        }
    }
}

// ==========================================
// 3. PHYSICS & SIMULATION
// ==========================================
pub mod physics {
    use super::*;
    use ::rand::Rng;

    pub struct Particle {
        pub pos: Vec3,
        pub radius: f32,
    }

    pub struct IceSimulation {
        pub particles: Vec<Particle>,
        pub mold: geometry::Mold,
        pub current_frame: usize,
        config: SimConfig,
    }

    impl IceSimulation {
        pub fn new(config: SimConfig) -> Self {
            Self {
                particles: Vec::new(),
                mold: geometry::Mold::default_crystal(),
                current_frame: 0,
                config,
            }
        }

        pub fn step(&mut self) {
            if self.current_frame < self.config.frames {
                self.spawn_particles();
                self.resolve_collisions_fast();
                self.current_frame += 1;
            }
        }

        fn spawn_particles(&mut self) {
            let mut rng = ::rand::thread_rng();
            for _ in 0..self.config.spawn_rate {
                let vertical_offset = rng.gen_range(-1.0..1.0);
                let pos = vec3(
                    rng.gen_range(-0.1..0.1),
                    vertical_offset,
                    rng.gen_range(-0.1..0.1),
                );
                let radius = rng.gen_range(self.config.radius_min..self.config.radius_max);
                self.particles.push(Particle { pos, radius });
            }
        }

        /// Spatial Hash Grid: Speeds up collision from O(N^2) to O(N)
        fn resolve_collisions_fast(&mut self) {
            let cell_size = self.config.radius_max * 2.0;

            for _ in 0..self.config.relaxation_steps {
                // 1. Build Grid
                let mut grid: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
                for (i, p) in self.particles.iter().enumerate() {
                    let cell = (
                        (p.pos.x / cell_size).floor() as i32,
                        (p.pos.y / cell_size).floor() as i32,
                        (p.pos.z / cell_size).floor() as i32,
                    );
                    grid.entry(cell).or_default().push(i);
                }

                // 2. Calculate displacements
                let mut displacements = vec![Vec3::ZERO; self.particles.len()];
                
                for i in 0..self.particles.len() {
                    let p1 = &self.particles[i];
                    let cell = (
                        (p1.pos.x / cell_size).floor() as i32,
                        (p1.pos.y / cell_size).floor() as i32,
                        (p1.pos.z / cell_size).floor() as i32,
                    );

                    // Check neighbors in 3x3x3 grid
                    for dx in -1..=1 {
                        for dy in -1..=1 {
                            for dz in -1..=1 {
                                let neighbor_cell = (cell.0 + dx, cell.1 + dy, cell.2 + dz);
                                if let Some(neighbors) = grid.get(&neighbor_cell) {
                                    for &j in neighbors {
                                        if i < j {
                                            let p2 = &self.particles[j];
                                            let dir = p1.pos - p2.pos;
                                            let dist_sq = dir.length_squared();
                                            let min_dist = p1.radius + p2.radius;

                                            if dist_sq < min_dist * min_dist && dist_sq > 0.00001 {
                                                let dist = dist_sq.sqrt();
                                                let overlap = min_dist - dist;
                                                // Dampened push for parallel stability
                                                let push = (dir / dist) * (overlap * 0.4); 
                                                displacements[i] += push;
                                                displacements[j] -= push;
                                            } else if dist_sq <= 0.00001 {
                                                // Jitter to prevent exact overlap explosions
                                                displacements[i] += vec3(0.001, 0.0, -0.001);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 3. Apply displacements and boundaries
                for i in 0..self.particles.len() {
                    self.particles[i].pos += displacements[i];
                    
                    // Extract the radius first to make the borrow checker happy!
                    let radius = self.particles[i].radius; 
                    self.mold.constrain(&mut self.particles[i].pos, radius);
                }
            }
        }

        pub fn export_obj(&self) {
            let filename = format!("final_ice_crystal_{}.obj", self.particles.len());
            let mut file = File::create(&filename).expect("Unable to create file");
            for p in &self.particles {
                writeln!(file, "v {} {} {}", p.pos.x, p.pos.y, p.pos.z).unwrap();
            }
            println!("Exported {} successfully!", filename);
        }
    }
}

// ==========================================
// 4. VISUALIZATION & MAIN ENGINE
// ==========================================

/// Generates a fast 3D point cloud mesh for Macroquad
fn build_point_cloud_mesh(particles: &[physics::Particle]) -> Mesh {
    let mut vertices = Vec::with_capacity(particles.len() * 4);
    let mut indices = Vec::with_capacity(particles.len() * 12);
    let color: [u8; 4] = Color::new(0.4, 0.8, 1.0, 1.0).into(); 

for (i, p) in particles.iter().enumerate() {
    let base_idx = (i * 4) as u16;
    let r = p.radius;
    
    // 2. Add `normal: Default::default()` to all four pushes!
    vertices.push(Vertex { position: p.pos + vec3(0., r, 0.), uv: vec2(0.,0.), color, normal: Default::default() });
    vertices.push(Vertex { position: p.pos + vec3(-r, -r, r), uv: vec2(0.,0.), color, normal: Default::default() });
    vertices.push(Vertex { position: p.pos + vec3(r, -r, r), uv: vec2(0.,0.), color, normal: Default::default() });
    vertices.push(Vertex { position: p.pos + vec3(0., -r, -r), uv: vec2(0.,0.), color, normal: Default::default() });
    
    indices.extend_from_slice(&[
            base_idx, base_idx+1, base_idx+2,
            base_idx, base_idx+2, base_idx+3,
            base_idx, base_idx+3, base_idx+1,
            base_idx+1, base_idx+3, base_idx+2,
        ]);
    }
    
    Mesh { vertices, indices, texture: None }
}


#[macroquad::main("Procedural Ice Visualizer")]
async fn main() {
    let config = SimConfig::parse();
    let mut sim = physics::IceSimulation::new(config.clone());

    // Orbital Camera Controls
    let mut cam_alpha: f32 = 0.5;
    let mut cam_beta: f32 = 0.5;
    let cam_distance: f32 = 4.0;
    let mut exported = false;

    loop {
        clear_background(Color::new(0.05, 0.05, 0.08, 1.0)); // Dark studio background

        // 1. Run Simulation Step
        sim.step();

        // 2. Camera Orbit Logic
        if is_mouse_button_down(MouseButton::Left) {
            let delta = mouse_delta_position();
            cam_alpha -= delta.x * 2.5;
            cam_beta += delta.y * 2.5;
            cam_beta = cam_beta.clamp(-1.5, 1.5); // Prevent flipping upside down
        }

        let cam_pos = vec3(
            cam_alpha.sin() * cam_beta.cos(),
            cam_beta.sin(),
            cam_alpha.cos() * cam_beta.cos(),
        ) * cam_distance;

        set_camera(&Camera3D {
            position: cam_pos,
            up: vec3(0.0, 1.0, 0.0),
            target: vec3(0.0, 0.0, 0.0),
            ..Default::default()
        });

        // 3. Render Particles
        let mesh = build_point_cloud_mesh(&sim.particles);
        draw_mesh(&mesh);

        // 4. Draw UI overlays
        set_default_camera(); // Switch back to 2D for UI text
        
        draw_text("PROCEDURAL ICE GENERATOR", 20.0, 30.0, 30.0, WHITE);
        draw_text(&format!("Frame: {} / {}", sim.current_frame, config.frames), 20.0, 60.0, 20.0, LIGHTGRAY);
        draw_text(&format!("Particles: {}", sim.particles.len()), 20.0, 85.0, 20.0, LIGHTGRAY);
        draw_text("Left Click + Drag to Rotate Camera", 20.0, 110.0, 20.0, YELLOW);

        if sim.current_frame == config.frames {
            draw_text("SIMULATION COMPLETE. Press [E] to Export OBJ.", 20.0, 145.0, 20.0, GREEN);
            if is_key_pressed(KeyCode::E) && !exported {
                sim.export_obj();
                exported = true;
            }
            if exported {
                draw_text("Export Saved to Project Folder!", 20.0, 175.0, 20.0, BLUE);
            }
        }

        next_frame().await;
    }
}