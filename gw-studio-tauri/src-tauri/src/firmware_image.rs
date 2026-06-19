use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub(crate) fn copy_file_prefix(source: &Path, destination: &Path, max_bytes: u64) -> Result<(), String> {
    let input = fs::File::open(source)
        .map_err(|error| format!("failed to open {}: {error}", source.display()))?;
    let mut limited = input.take(max_bytes);
    let mut output = fs::File::create(destination)
        .map_err(|error| format!("failed to create {}: {error}", destination.display()))?;
    std::io::copy(&mut limited, &mut output)
        .map_err(|error| format!("failed to copy {} prefix: {error}", source.display()))?;
    Ok(())
}

pub(crate) fn compose_spi_image(
    stock_spi_path: &Path,
    retro_go_extflash_path: &Path,
    destination: &Path,
    offset_bytes: u64,
) -> Result<(), String> {
    copy_file_prefix(stock_spi_path, destination, offset_bytes)?;
    if !retro_go_extflash_path.is_file() {
        return Ok(());
    }
    let mut output = fs::OpenOptions::new()
        .write(true)
        .open(destination)
        .map_err(|error| format!("failed to open {} for SPI composition: {error}", destination.display()))?;
    output
        .seek(SeekFrom::Start(offset_bytes))
        .map_err(|error| format!("failed to seek {}: {error}", destination.display()))?;
    let mut payload = fs::File::open(retro_go_extflash_path)
        .map_err(|error| format!("failed to open {}: {error}", retro_go_extflash_path.display()))?;
    std::io::copy(&mut payload, &mut output)
        .map_err(|error| format!("failed to append Retro-Go fork SPI payload: {error}"))?;
    Ok(())
}
