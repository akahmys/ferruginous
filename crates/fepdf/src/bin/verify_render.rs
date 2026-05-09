use clap::Parser;
use image::{GenericImageView, Pixel};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    expected: PathBuf,

    #[arg(short, long)]
    actual: PathBuf,

    /// Maximum allowed difference ratio (0.0 to 1.0)
    #[arg(short, long, default_value_t = 0.05)]
    tolerance: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("Verifying render...");
    println!("Expected: {}", args.expected.display());
    println!("Actual:   {}", args.actual.display());

    let img_expected = image::open(&args.expected)?;
    let img_actual = image::open(&args.actual)?;

    if (img_expected.width() as i32 - img_actual.width() as i32).abs() > 2 || 
       (img_expected.height() as i32 - img_actual.height() as i32).abs() > 2 {
        println!("FAIL: Image dimensions differ too much.");
        println!("  Expected: {:?}", img_expected.dimensions());
        println!("  Actual:   {:?}", img_actual.dimensions());
        std::process::exit(1);
    }

    let width = img_expected.width().min(img_actual.width());
    let height = img_expected.height().min(img_actual.height());
    let mut diff_pixels = 0;
    let total_pixels = width * height;

    for y in 0..height {
        for x in 0..width {
            let p_exp = img_expected.get_pixel(x, y).to_rgba();
            let p_act = img_actual.get_pixel(x, y).to_rgba();

            // Simple distance
            let dist = (p_exp[0] as i32 - p_act[0] as i32).abs()
                + (p_exp[1] as i32 - p_act[1] as i32).abs()
                + (p_exp[2] as i32 - p_act[2] as i32).abs();

            if dist > 50 {
                diff_pixels += 1;
            }
        }
    }

    let diff_ratio = diff_pixels as f64 / total_pixels as f64;
    println!("Difference: {:.2}% ({} / {} pixels)", diff_ratio * 100.0, diff_pixels, total_pixels);

    if diff_ratio > args.tolerance {
        println!("FAIL: Images differ by more than tolerance ({:.2}% > {:.2}%)", diff_ratio * 100.0, args.tolerance * 100.0);
        std::process::exit(1);
    }

    println!("PASS: Images match within tolerance.");
    Ok(())
}
