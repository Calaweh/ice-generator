use glam::Vec3;
use rand::Rng;
use std::fs::File;
use std::io::Write;

struct Plane {
    normal: Vec3,
    distance: f32,
}

struct Mold {
    planes: Vec<Plane>,
}

impl Mold {
    fn new(planes: Vec<Plane>) -> Self {
        Self { planes }
    }

    fn constrain(&self, pos: &mut Vec3, radius: f32) {
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

struct Particle {
    pos: Vec3,
    radius: f32,
}

struct IceSimulation {
    particles: Vec<Particle>,
    mold: Mold,
}

impl IceSimulation {
    fn new(mold: Mold) -> Self {
        Self {
            particles: Vec::new(),
            mold,
        }
    }

    fn grow_particles(&mut self, count: usize) {
        let mut rng = rand::thread_rng();
        for _ in 0..count {
            // NEW: Spawn along a vertical "spine" instead of a single point.
            // This helps fill the tall, pointy shard shape much better.
            let vertical_offset = rng.gen_range(-1.0..1.0); 
            
            let spawn_pos = Vec3::new(
                rng.gen_range(-0.1..0.1),
                vertical_offset, // Spread out up and down
                rng.gen_range(-0.1..0.1),
            );
            
            // Change it to this:
let radius = rng.gen_range(0.02..0.06);
            
            self.particles.push(Particle {
                pos: spawn_pos,
                radius,
            });
        }
    }

    fn resolve_pressure(&mut self) {
        let num_particles = self.particles.len();
        let relaxation_steps = 3;

        for _ in 0..relaxation_steps {
            for i in 0..num_particles {
                for j in (i + 1)..num_particles {
                    let dir = self.particles[i].pos - self.particles[j].pos;
                    let dist_sq = dir.length_squared();
                    let min_dist = self.particles[i].radius + self.particles[j].radius;

                    if dist_sq < min_dist * min_dist && dist_sq > 0.00001 {
                        let dist = dist_sq.sqrt();
                        let overlap = min_dist - dist;
                        let push_dir = dir / dist;
                        let push_vector = push_dir * (overlap * 0.5);
                        
                        self.particles[i].pos += push_vector;
                        self.particles[j].pos -= push_vector;
                    }
                }
            }

            for particle in &mut self.particles {
                self.mold.constrain(&mut particle.pos, particle.radius);
            }
        }
    }

    fn export_frame_to_obj(&self, frame_number: usize) {
        let filename = format!("frame_{:04}.obj", frame_number);
        let mut file = File::create(&filename).expect("Unable to create file");

        for particle in &self.particles {
            writeln!(file, "v {} {} {}", particle.pos.x, particle.pos.y, particle.pos.z)
                .expect("Unable to write data");
        }
        println!("Exported {} with {} particles", filename, self.particles.len());
    }
}

fn main() {
    let mut planes = Vec::new();

    // 1. Thickness (Make it a relatively thin slab like the reference)
    planes.push(Plane { normal: Vec3::new(0.0, 0.0, 1.0).normalize(), distance: 0.6 });
    planes.push(Plane { normal: Vec3::new(0.0, 0.0, -1.0).normalize(), distance: 0.6 });

    // 2. Main Width Limits
    planes.push(Plane { normal: Vec3::new(-1.0, 0.0, 0.0).normalize(), distance: 1.1 });
    planes.push(Plane { normal: Vec3::new(1.0, 0.0, 0.0).normalize(), distance: 1.1 });

    // 3. Top Profile (Slanted heavily down to the left)
    planes.push(Plane { normal: Vec3::new(-0.4, 1.0, 0.0).normalize(), distance: 1.6 }); 
    planes.push(Plane { normal: Vec3::new(0.6, 1.0, 0.0).normalize(), distance: 1.8 }); // Snip the top right corner

    // 4. Bottom Profile (Tapered into an asymmetrical jagged point pointing down-right)
    planes.push(Plane { normal: Vec3::new(-1.0, -1.0, 0.0).normalize(), distance: 1.3 }); // Steep slope bottom left
    planes.push(Plane { normal: Vec3::new(0.8, -1.2, 0.0).normalize(), distance: 1.2 }); // Sharp cut bottom right

    // 5. Faceted 3D corners (These "carve" the block so it isn't just a flat extrusion)
    planes.push(Plane { normal: Vec3::new(-0.8, -0.8, 0.6).normalize(), distance: 1.1 }); // Bottom left front chip
    planes.push(Plane { normal: Vec3::new(0.8, 0.8, 0.6).normalize(), distance: 1.3 }); // Top right front chip
    planes.push(Plane { normal: Vec3::new(0.6, -0.8, -0.6).normalize(), distance: 1.1 }); // Bottom right back chip
    planes.push(Plane { normal: Vec3::new(-0.6, 0.8, -0.6).normalize(), distance: 1.3 }); // Top left back chip

    let mold = Mold::new(planes);
    let mut sim = IceSimulation::new(mold);

    // Change it to this:
let total_frames = 150;
let growth_per_frame = 150;

    for frame in 0..total_frames {
        sim.grow_particles(growth_per_frame);
        sim.resolve_pressure();
        sim.export_frame_to_obj(frame);
    }
}