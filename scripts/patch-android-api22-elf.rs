use std::{env, fs, path::Path};

const PT_DYNAMIC: u32 = 2;
const DT_NULL: u32 = 0;
const DT_HASH: u32 = 4;
const DT_GNU_HASH: u32 = 0x6fff_fef5;
const DT_VERSYM: u32 = 0x6fff_fff0;
const DT_FLAGS_1: u32 = 0x6fff_fffb;
const DT_VERNEED: u32 = 0x6fff_fffe;
const DT_VERNEEDNUM: u32 = 0x6fff_ffff;
const DF_1_PIE: u32 = 0x0800_0000;

fn read_u16(data: &[u8], offset: usize) -> Result<u16, String> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| format!("truncated ELF at offset {offset}"))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, String> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| format!("truncated ELF at offset {offset}"))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn write_u32(data: &mut [u8], offset: usize, value: u32) -> Result<(), String> {
    let bytes = data
        .get_mut(offset..offset + 4)
        .ok_or_else(|| format!("truncated ELF at offset {offset}"))?;
    bytes.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn patch(path: &Path) -> Result<(), String> {
    let mut data = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if data.get(..6) != Some(b"\x7fELF\x01\x01") {
        return Err("expected a 32-bit little-endian ELF".into());
    }
    if read_u16(&data, 16)? != 3 || read_u16(&data, 18)? != 40 {
        return Err("expected an ARM position-independent executable".into());
    }

    let phoff = read_u32(&data, 28)? as usize;
    let phentsize = read_u16(&data, 42)? as usize;
    let phnum = read_u16(&data, 44)? as usize;
    if phentsize < 20 {
        return Err("invalid ELF program header size".into());
    }

    let mut dynamic = None;
    for index in 0..phnum {
        let header = phoff
            .checked_add(
                index
                    .checked_mul(phentsize)
                    .ok_or("program header overflow")?,
            )
            .ok_or("program header overflow")?;
        if read_u32(&data, header)? == PT_DYNAMIC {
            dynamic = Some((
                read_u32(&data, header + 4)? as usize,
                read_u32(&data, header + 16)? as usize,
            ));
            break;
        }
    }
    let (offset, size) = dynamic.ok_or("missing PT_DYNAMIC")?;
    if size < 8
        || size % 8 != 0
        || offset
            .checked_add(size)
            .filter(|end| *end <= data.len())
            .is_none()
    {
        return Err("invalid PT_DYNAMIC bounds".into());
    }

    let mut entries = Vec::new();
    for position in (offset..offset + size).step_by(8) {
        let tag = read_u32(&data, position)?;
        let value = read_u32(&data, position + 4)?;
        if tag == DT_NULL {
            break;
        }
        entries.push((tag, value));
    }

    if !entries.iter().any(|(tag, _)| *tag == DT_HASH) {
        return Err("no SysV DT_HASH table to retain".into());
    }
    let ignored = [DT_GNU_HASH, DT_VERSYM, DT_VERNEED, DT_VERNEEDNUM];
    let flags_1 = entries
        .iter()
        .find(|(tag, _)| *tag == DT_FLAGS_1)
        .map_or(0, |(_, value)| *value);
    if !entries.iter().any(|(tag, _)| ignored.contains(tag)) && flags_1 & DF_1_PIE == 0 {
        println!(
            "Android 5 linker metadata is already clean: {}",
            path.display()
        );
        return Ok(());
    }

    let mut cleaned = Vec::with_capacity(entries.len());
    for (tag, mut value) in entries {
        if tag == DT_FLAGS_1 {
            value &= !DF_1_PIE;
        }
        if !ignored.contains(&tag) {
            cleaned.push((tag, value));
        }
    }
    if cleaned.len() >= size / 8 {
        return Err("dynamic section has no room for DT_NULL".into());
    }

    data[offset..offset + size].fill(0);
    for (index, (tag, value)) in cleaned.iter().enumerate() {
        let position = offset + index * 8;
        write_u32(&mut data, position, *tag)?;
        write_u32(&mut data, position + 4, *value)?;
    }
    fs::write(path, data).map_err(|error| format!("{}: {error}", path.display()))?;
    println!("Cleaned Android 5 linker metadata: {}", path.display());
    Ok(())
}

fn main() {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 2 {
        eprintln!("Usage: patch-android-api22-elf <armv7 ELF>");
        std::process::exit(2);
    }
    if let Err(error) = patch(Path::new(&args[1])) {
        eprintln!("patch-android-api22-elf: {error}");
        std::process::exit(1);
    }
}
