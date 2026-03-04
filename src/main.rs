use glam::Vec3;
use noise::{NoiseFn, Perlin};
use rand::Rng;
use std::fs::File;
use std::io::Write;

// --- TOOL SETTINGS ---
const RESOLUTION: usize = 128; // Grid density. 128^3 = ~2 million voxels.
const MOLD_DIM: f32 = 1.0;     // The boundary (-1.0 to 1.0)
const CAVE_SIZE: f64 = 1.2;    // Scale of internal air pockets
const AIR_THRESHOLD: f32 = 0.05; // Higher = more air holes, Lower = more solid ice

fn main() -> std::io::Result<()> {
    let mut rng = rand::thread_rng();
    let perlin = Perlin::new(1337);
    let mut particles = Vec::new();

    println!("🧊 Simulating Volumetric Ice Growth with Internal Air Voids...");

    // 1. Iterate through the entire 3D VOLUME
    for z in 0..RESOLUTION {
        for y in 0..RESOLUTION {
            for x in 0..RESOLUTION {
                // Map grid (0..RES) to coordinates (-1.0 .. 1.0)
                let p = Vec3::new(
                    (x as f32 / RESOLUTION as f32) * 2.0 - 1.0,
                    (y as f32 / RESOLUTION as f32) * 2.0 - 1.0,
                    (z as f32 / RESOLUTION as f32) * 2.0 - 1.0,
                );

                // 2. NUCLEATION LOGIC (Noise-based Freezing)
                let noise_val = perlin.get([
                    p.x as f64 * CAVE_SIZE,
                    p.y as f64 * CAVE_SIZE,
                    p.z as f64 * CAVE_SIZE,
                ]) as f32;

                // Only create ice where noise is above threshold
                // This creates the "Natural Voids" (Air Holes) inside the block
                if noise_val > AIR_THRESHOLD {
                    
                    // 3. STOCHASTIC JITTER (Micro-Texture)
                    // We offset the point slightly so they aren't a perfect grid.
                    // This creates the "uneven tiny dense particles" in Blender.
                    let jitter = Vec3::new(
                        rng.gen_range(-0.4..0.4),
                        rng.gen_range(-0.4..0.4),
                        rng.gen_range(-0.4..0.4),
                    ) * (2.0 / RESOLUTION as f32);

                    let mut final_pos = p + jitter;

                    // 4. THE MOLD BOUNDARY (Even Surface)
                    // Clamp ensures we "hit" the glass wall and flatten out perfectly.
                    final_pos.x = final_pos.x.clamp(-MOLD_DIM, MOLD_DIM);
                    final_pos.y = final_pos.y.clamp(-MOLD_DIM, MOLD_DIM);
                    final_pos.z = final_pos.z.clamp(-MOLD_DIM, MOLD_DIM);

                    // OPTIMIZATION: 
                    // We only save particles near the surface OR near an air-pocket.
                    // This keeps the file size manageable while keeping the "refraction walls."
                    if noise_val < AIR_THRESHOLD + 0.1 || final_pos.x.abs() > 0.98 || final_pos.y.abs() > 0.98 || final_pos.z.abs() > 0.98 {
                         particles.push(final_pos);
                    }
                }
            }
        }
    }

    // 5. EXPORT (Fixed Ownership)
    let mut file = File::create("abeto_volume_ice.obj")?;
    
    // We use '&particles' here so we only BORROW the vector
    for p in &particles {
        // Multiply by 10 for Blender scale
        writeln!(file, "v {} {} {}", p.x * 10.0, p.y * 10.0, p.z * 10.0)?;
    }

    println!("✅ Done! Generated {} points.", particles.len());
    println!("👉 Import 'abeto_volume_ice.obj' into Blender.");
    Ok(())
}