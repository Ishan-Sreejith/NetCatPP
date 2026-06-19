use anyhow::{Context, Result, bail};
use camino::Utf8PathBuf;
use uniffi_bindgen::bindings::SwiftBindingGenerator;
use uniffi_bindgen::library_mode;
use uniffi_bindgen::EmptyCrateConfigSupplier;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let library = args.next().context("missing library path")?;
    let out_dir = args.next().context("missing out-dir path")?;
    if args.next().is_some() {
        bail!("usage: uniffi-gen <library-path> <out-dir>");
    }

    let library = Utf8PathBuf::from(library);
    let out_dir = Utf8PathBuf::from(out_dir);

    let generator = SwiftBindingGenerator;
    let supplier = EmptyCrateConfigSupplier;

    library_mode::generate_bindings(
        &library,
        None,
        &generator,
        &supplier,
        None,
        &out_dir,
        true,
    )?;

    println!("Generated Swift bindings at {}", out_dir);
    Ok(())
}
