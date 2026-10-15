//! Every GPU on Windows (Intel, AMD, and NVIDIA without `nvidia-smi`) from the GPU
//! performance counters, the ones Task Manager shows. The adapters (names, VRAM)
//! come from DXGI and are matched to the counters by their LUID.
//!
//! - Usage: per adapter, the busiest kind of engine (`3D`, `Copy`, `VideoDecode`, …),
//!   summed over the processes using it, as Task Manager does.
//! - Memory: dedicated VRAM; GPUs with little or no VRAM of their own (integrated)
//!   report what they use of the system memory, against the whole of it.
//!
//! Counter rates need two samples, so the first sample reports every GPU as idle.

use std::{
    collections::HashMap,
    ptr,
    sync::{Mutex, OnceLock},
};

use windows_sys::Win32::System::{
    Performance::{
        PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
        PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA, PdhAddEnglishCounterW, PdhCollectQueryData,
        PdhGetFormattedCounterArrayW, PdhOpenQueryW,
    },
    SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
};

use crate::status::{GpuData, GpuMemory};

const MIB: u64 = 1024 * 1024;
/// At most this much VRAM of its own is an integrated GPU's carve-out, not real VRAM
const INTEGRATED_VRAM_MIB: u64 = 512;
const NVIDIA: u32 = 0x10de;
/// Microsoft's Basic Render Driver and remote display adapters
const MICROSOFT: u32 = 0x1414;

pub fn get_info(skip_nvidia: bool) -> Vec<GpuData> {
    static ADAPTERS: OnceLock<Vec<Adapter>> = OnceLock::new();
    // Opened once; when that fails (counters disabled), not tried again
    static COUNTERS: OnceLock<Option<Mutex<Counters>>> = OnceLock::new();

    let adapters: Vec<&Adapter> = ADAPTERS
        .get_or_init(dxgi::adapters)
        .iter()
        .filter(|a| a.vendor != MICROSOFT && !(skip_nvidia && a.vendor == NVIDIA))
        .collect();
    if adapters.is_empty() {
        return Vec::new();
    }

    let sample = COUNTERS
        .get_or_init(|| Counters::open().map(Mutex::new))
        .as_ref()
        .map(|counters| {
            counters
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .sample()
        })
        .unwrap_or_default();
    let ram_mib = ram_mib();

    adapters
        .iter()
        .map(|adapter| {
            let usage = sample.usage(adapter.luid);
            let (dedicated, shared) = sample.memory.get(&adapter.luid).copied().unwrap_or((0, 0));
            let memory = if adapter.vram_mib > INTEGRATED_VRAM_MIB {
                GpuMemory {
                    free: adapter.vram_mib.saturating_sub(dedicated / MIB),
                    total: adapter.vram_mib,
                }
            } else if let Some(total) = ram_mib {
                GpuMemory {
                    free: total.saturating_sub((dedicated + shared) / MIB),
                    total,
                }
            } else {
                GpuMemory { free: 0, total: 0 }
            };
            GpuData {
                name: adapter.name.clone(),
                usage,
                memory,
                temperature: None,
                usage_known: true,
            }
        })
        .collect()
}

fn ram_mib() -> Option<u64> {
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = u32::try_from(size_of::<MEMORYSTATUSEX>()).ok()?;
    // SAFETY: the pointer is to a MEMORYSTATUSEX with its length set, alive for the call
    (unsafe { GlobalMemoryStatusEx(&raw mut status) } != 0).then_some(status.ullTotalPhys / MIB)
}

/// An adapter's LUID as the counters' instance names write it: (high part, low part).
type Luid = (u32, u32);

#[derive(Clone, Debug)]
struct Adapter {
    name: String,
    vendor: u32,
    luid: Luid,
    vram_mib: u64,
}

struct Counters {
    query: PDH_HQUERY,
    engine: PDH_HCOUNTER,
    dedicated: PDH_HCOUNTER,
    shared: PDH_HCOUNTER,
}

// SAFETY: PDH handles may be used from any thread; they are only used behind a Mutex
unsafe impl Send for Counters {}

#[derive(Default)]
struct Sample {
    /// (adapter, engine type) → summed utilization
    engines: HashMap<(Luid, String), f64>,
    /// adapter → (dedicated, shared) bytes in use
    memory: HashMap<Luid, (u64, u64)>,
}

impl Sample {
    fn usage(&self, luid: Luid) -> f64 {
        self.engines
            .iter()
            .filter(|((l, _), _)| *l == luid)
            .map(|(_, &v)| v)
            .fold(0.0, f64::max)
            .clamp(0.0, 100.0)
    }
}

impl Counters {
    fn open() -> Option<Self> {
        let mut query: PDH_HQUERY = ptr::null_mut();
        // SAFETY: the out pointer is to a local handle
        if unsafe { PdhOpenQueryW(ptr::null(), 0, &raw mut query) } != 0 {
            return None;
        }
        let add = |path: &str| {
            let path: Vec<u16> = path.encode_utf16().chain([0]).collect();
            let mut counter: PDH_HCOUNTER = ptr::null_mut();
            // SAFETY: the path is NUL-terminated and outlives the call; the out pointer is local
            (unsafe { PdhAddEnglishCounterW(query, path.as_ptr(), 0, &raw mut counter) } == 0)
                .then_some(counter)
        };
        Some(Self {
            engine: add(r"\GPU Engine(*)\Utilization Percentage")?,
            dedicated: add(r"\GPU Adapter Memory(*)\Dedicated Usage")?,
            shared: add(r"\GPU Adapter Memory(*)\Shared Usage")?,
            query,
        })
    }

    fn sample(&self) -> Sample {
        let mut sample = Sample::default();
        // SAFETY: the query handle is open for the life of the process
        if unsafe { PdhCollectQueryData(self.query) } != 0 {
            return sample;
        }
        for (instance, value) in values(self.engine) {
            if let Some((luid, engine)) = parse_engine(&instance) {
                *sample.engines.entry((luid, engine.to_owned())).or_default() += value;
            }
        }
        for (counter, shared) in [(self.dedicated, false), (self.shared, true)] {
            for (instance, value) in values(counter) {
                if let Some(luid) = parse_luid(&instance) {
                    let entry = sample.memory.entry(luid).or_default();
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let bytes = value.max(0.0) as u64;
                    if shared {
                        entry.1 += bytes;
                    } else {
                        entry.0 += bytes;
                    }
                }
            }
        }
        sample
    }
}

/// Every instance of a wildcard counter with its value, from the last collection.
fn values(counter: PDH_HCOUNTER) -> Vec<(String, f64)> {
    let mut size = 0u32;
    let mut count = 0u32;
    // SAFETY: asks for the buffer size only
    let status = unsafe {
        PdhGetFormattedCounterArrayW(
            counter,
            PDH_FMT_DOUBLE,
            &raw mut size,
            &raw mut count,
            ptr::null_mut(),
        )
    };
    if status != PDH_MORE_DATA || size == 0 {
        return Vec::new();
    }
    // u64s keep the items (pointers and doubles) aligned
    let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
    let items = buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
    // SAFETY: the buffer holds `size` bytes, as PDH asked for
    let status = unsafe {
        PdhGetFormattedCounterArrayW(
            counter,
            PDH_FMT_DOUBLE,
            &raw mut size,
            &raw mut count,
            items,
        )
    };
    if status != 0 {
        return Vec::new();
    }
    // SAFETY: PDH wrote `count` items to the buffer; their names point into it
    let items = unsafe { std::slice::from_raw_parts(items, count as usize) };
    items
        .iter()
        .filter(|item| {
            matches!(
                item.FmtValue.CStatus,
                PDH_CSTATUS_VALID_DATA | PDH_CSTATUS_NEW_DATA
            )
        })
        .map(|item| {
            // SAFETY: szName is a NUL-terminated string inside the buffer
            let name = unsafe { wide_string(item.szName) };
            // SAFETY: PDH_FMT_DOUBLE fills the double
            let value = unsafe { item.FmtValue.Anonymous.doubleValue };
            (name, value)
        })
        .collect()
}

/// # Safety
/// `text` is null or points to a NUL-terminated UTF-16 string.
unsafe fn wide_string(text: *const u16) -> String {
    if text.is_null() {
        return String::new();
    }
    let mut len = 0;
    // SAFETY: the string ends with a NUL, so every unit up to it is readable
    while unsafe { *text.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: `len` units were just read
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, len) })
}

/// `…_luid_0x00000000_0x0000C2B4_…` → (0, 0xC2B4).
fn parse_luid(instance: &str) -> Option<Luid> {
    let rest = &instance[instance.find("luid_0x")? + "luid_0x".len()..];
    let (high, rest) = rest.split_once("_0x")?;
    let low: String = rest.chars().take_while(char::is_ascii_hexdigit).collect();
    Some((
        u32::from_str_radix(high, 16).ok()?,
        u32::from_str_radix(&low, 16).ok()?,
    ))
}

/// `pid_1234_luid_0x00000000_0x0000C2B4_phys_0_eng_0_engtype_3D` → (LUID, `3D`).
fn parse_engine(instance: &str) -> Option<(Luid, &str)> {
    let luid = parse_luid(instance)?;
    let engine = &instance[instance.rfind("engtype_")? + "engtype_".len()..];
    Some((luid, engine))
}

mod dxgi {
    use std::{ffi::c_void, ptr};

    use super::{Adapter, MIB};

    #[repr(C)]
    struct Guid(u32, u16, u16, [u8; 8]);

    static IID_IDXGI_FACTORY1: Guid = Guid(
        0x770a_ae78,
        0xf26f,
        0x4dba,
        [0xa8, 0x29, 0x25, 0x3c, 0x83, 0xd1, 0xb3, 0x87],
    );
    const DXGI_ADAPTER_FLAG_SOFTWARE: u32 = 2;

    /// `DXGI_ADAPTER_DESC1`
    #[repr(C)]
    struct AdapterDesc {
        description: [u16; 128],
        vendor_id: u32,
        device_id: u32,
        sub_sys_id: u32,
        revision: u32,
        dedicated_video_memory: usize,
        dedicated_system_memory: usize,
        shared_system_memory: usize,
        luid_low: u32,
        luid_high: i32,
        flags: u32,
    }

    #[link(name = "dxgi")]
    unsafe extern "system" {
        fn CreateDXGIFactory1(riid: *const Guid, factory: *mut *mut c_void) -> i32;
    }

    // Slots in the COM vtables
    const RELEASE: usize = 2;
    /// `IDXGIFactory1::EnumAdapters1`
    const ENUM_ADAPTERS1: usize = 12;
    /// `IDXGIAdapter1::GetDesc1`
    const GET_DESC1: usize = 10;

    type Release = unsafe extern "system" fn(*mut c_void) -> u32;
    type EnumAdapters1 = unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> i32;
    type GetDesc1 = unsafe extern "system" fn(*mut c_void, *mut AdapterDesc) -> i32;

    /// # Safety
    /// `object` is a live COM object whose vtable has a `T` at `slot`.
    unsafe fn method<T: Copy>(object: *mut c_void, slot: usize) -> T {
        // SAFETY: a COM object starts with a pointer to its vtable of function pointers
        unsafe {
            let vtable = *object.cast::<*const usize>();
            std::mem::transmute_copy(&*vtable.add(slot))
        }
    }

    /// The hardware adapters, without DXGI's software renderer.
    pub fn adapters() -> Vec<Adapter> {
        let mut factory: *mut c_void = ptr::null_mut();
        // SAFETY: the IID matches the interface asked for; the out pointer is local
        if unsafe { CreateDXGIFactory1(&raw const IID_IDXGI_FACTORY1, &raw mut factory) } < 0
            || factory.is_null()
        {
            return Vec::new();
        }
        let mut adapters = Vec::new();
        // SAFETY: `factory` is an IDXGIFactory1 until released below; each adapter is
        // released after its description is read
        unsafe {
            let enum_adapters: EnumAdapters1 = method(factory, ENUM_ADAPTERS1);
            for index in 0.. {
                let mut adapter: *mut c_void = ptr::null_mut();
                // Fails with DXGI_ERROR_NOT_FOUND after the last one
                if enum_adapters(factory, index, &raw mut adapter) < 0 || adapter.is_null() {
                    break;
                }
                let get_desc: GetDesc1 = method(adapter, GET_DESC1);
                let mut desc: AdapterDesc = std::mem::zeroed();
                if get_desc(adapter, &raw mut desc) >= 0
                    && desc.flags & DXGI_ADAPTER_FLAG_SOFTWARE == 0
                {
                    let len = desc.description.iter().position(|&c| c == 0).unwrap_or(128);
                    adapters.push(Adapter {
                        name: String::from_utf16_lossy(&desc.description[..len])
                            .trim()
                            .to_owned(),
                        vendor: desc.vendor_id,
                        luid: (desc.luid_high.cast_unsigned(), desc.luid_low),
                        vram_mib: desc.dedicated_video_memory as u64 / MIB,
                    });
                }
                let release: Release = method(adapter, RELEASE);
                release(adapter);
            }
            let release: Release = method(factory, RELEASE);
            release(factory);
        }
        adapters
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_engine, parse_luid};

    #[test]
    fn reads_instance_names() {
        assert_eq!(
            parse_engine("pid_1234_luid_0x00000000_0x0000C2B4_phys_0_eng_0_engtype_3D"),
            Some(((0, 0xC2B4), "3D"))
        );
        assert_eq!(
            parse_engine("pid_8_luid_0x00000001_0x0001A2B3_phys_0_eng_5_engtype_VideoDecode"),
            Some(((1, 0x1_A2B3), "VideoDecode"))
        );
        assert_eq!(
            parse_luid("luid_0x00000000_0x0000C2B4_phys_0"),
            Some((0, 0xC2B4))
        );
        assert_eq!(parse_luid("_Total"), None);
    }
}
