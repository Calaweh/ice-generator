use glam::Vec3;
use noise::{NoiseFn, Perlin, Seedable};
use std::fs::File;
use std::io::Write;

fn main() -> std::io::Result<()> {
    // 1. TOOL SETTINGS (Adjust these to get the "Case Study" look)
    let resolution = 128;   // High detail for micro-holes. (Start at 64 if slow)
    let mold_limit = 0.98;  // The "Glass Wall" (shaves the ice at this distance)
    let growth_scale = 2.5; // Big organic lumps
    let frost_scale = 80.0; // Micro-pitting (Frosted texture)
    
    let perlin = Perlin::new(42); // Seed for the ice growth

    println!("🧊 Generating Ice with resolution {}x{}...", resolution, resolution);

    // We store our "Ice Density" in a 1D vector (simulating a 3D grid)
    let mut grid = vec![0.0f32; resolution * resolution * resolution];

    // --- PHASE 1: NUCLEATION & GROWTH ---
    for z in 0..resolution {
        for y in 0..resolution {
            for x in 0..resolution {
                let idx = x + y * resolution + z * resolution * resolution;
                
                // Map grid (0..res) to coordinates (-1.0 .. 1.0)
                let p = Vec3::new(
                    (x as f32 / resolution as f32) * 2.0 - 1.0,
                    (y as f32 / resolution as f32) * 2.0 - 1.0,
                    (z as f32 / resolution as f32) * 2.0 - 1.0,
                );

                // THE ABETO LOGIC:
                // 1. Calculate Organic Shape
                let noise_val = perlin.get([
                    (p.x * growth_scale) as f64, 
                    (p.y * growth_scale) as f64, 
                    (p.z * growth_scale) as f64
                ]) as f32;

                // 2. Add Tiny Micro-Holes
                let frost = perlin.get([
                    (p.x * frost_scale) as f64, 
                    (p.y * frost_scale) as f64, 
                    (p.z * frost_scale) as f64
                ]) as f32 * 0.1;

                // 3. The "Mold" Constraint (SDF Box)
                // If the coordinate is outside 0.98, we kill the growth (The Shave)
                let inside_mold = p.abs().max_element() < mold_limit;
                
                if inside_mold && (noise_val + frost) > 0.1 {
                    grid[idx] = 1.0; // Mark as SOLID ICE
                }
            }
        }
    }

    // --- PHASE 2: SURFACE EXPORT (Creating the .obj) ---
    let mut file = File::create("ice_block.obj")?;
    let mut vertex_count = 0;

    println!("💾 Exporting to ice_block.obj...");

    for z in 1..resolution-1 {
        for y in 1..resolution-1 {
            for x in 1..resolution-1 {
                let idx = x + y * resolution + z * resolution * resolution;
                
                if grid[idx] > 0.5 {
                    // Check if this voxel is on the SURFACE 
                    // (Does it have an empty neighbor?)
                    let neighbors = [
                        grid[idx + 1], grid[idx - 1],
                        grid[idx + resolution], grid[idx - resolution],
                        grid[idx + resolution * resolution], grid[idx - resolution * resolution],
                    ];

                    if neighbors.iter().any(|&v| v < 0.5) {
                        // It's a surface voxel! Export a tiny cube here.
                        let p = Vec3::new(x as f32, y as f32, z as f32);
                        
                        // Write a vertex to the OBJ file
                        writeln!(file, "v {} {} {}", p.x, p.y, p.z)?;
                        vertex_count += 1;
                    }
                }
            }
        }
    }

    println!("✅ Done! Generated {} surface points.", vertex_count);
    println!("👉 Drag 'ice_block.obj' into Blender to see the results.");
    Ok(())
}