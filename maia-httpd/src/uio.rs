//! UIO device access.
//!
//! This module is used to work with UIO devices.

use anyhow::Result;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// UIO device.
///
/// This struct represents an UIO device.
#[derive(Debug)]
pub struct Uio {
    num: usize,
    file: fs::File,
}

/// UIO device mapping.
///
/// This struct corresponds to a memory-mapped IO region of an UIO device, and
/// gives access to the region. Dropping this struct unmaps the region.
#[derive(Debug, Clone)]
pub struct Mapping {
    base: *mut libc::c_void,
    effective: *mut libc::c_void,
    map_size: usize,
}

impl Uio {
    /// Opens an UIO using its number.
    ///
    /// This function opens the UIO device `/dev/uio<num>`, where `num` is the
    /// parameter indicating the UIO device number.
    pub async fn from_num(num: usize) -> Result<Uio> {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!("/dev/uio{num}"))
            .await?;
        Ok(Uio { num, file })
    }

    /// Opens an UIO using its name.
    ///
    /// This function searches in `/sys/class/uio` an UIO device whose name
    /// matches the indicated one and opens it.
    pub async fn from_name(name: &str) -> Result<Uio> {
        match Self::find_by_name(name).await? {
            Some(num) => Self::from_num(num).await,
            None => anyhow::bail!("UIO device not found"),
        }
    }

    async fn find_by_name(name: &str) -> Result<Option<usize>> {
        let mut entries = fs::read_dir(Path::new("/sys/class/uio")).await?;
        while let Some(entry) = entries.next_entry().await? {
            let file_name = entry.file_name();
            let uio = file_name
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("file name is not valid UTF8"))?;
            if let Some(num) = uio
                .strip_prefix("uio")
                .and_then(|a| a.parse::<usize>().ok())
            {
                let mut path = entry.path();
                path.push("name");
                let this_name = fs::read_to_string(path).await?;
                if this_name.trim_end() == name {
                    return Ok(Some(num));
                }
            }
        }
        Ok(None)
    }

    /// Maps a memory mapping of an UIO device.
    ///
    /// The `mapping` number is the number that corresponds to the mapping, as
    /// listed in `/sys/class/uio/uio*/maps/map<mapping>`. Mappings are numbered
    /// sequentially for each device, so devices that only support one mapping
    /// use `0` as the value for `mapping`.
    pub async fn map_mapping(&self, mapping: usize) -> Result<Mapping> {
        let offset = mapping * page_size::get();
        let fd = self.file.as_raw_fd();
        let map_size = self.map_size(mapping).await?;

        let base = unsafe {
            match libc::mmap(
                std::ptr::null_mut::<libc::c_void>(),
                map_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                offset as libc::off_t,
            ) {
                libc::MAP_FAILED => anyhow::bail!("mmap UIO failed"),
                x => x,
            }
        };
        let effective_offset = isize::try_from(self.map_offset(mapping).await?)?;
        let effective = unsafe { base.offset(effective_offset) };
        Ok(Mapping {
            base,
            effective,
            map_size,
        })
    }

    async fn read_mapping_hex(&self, mapping: usize, fname: &str) -> Result<usize> {
        let n = fs::read_to_string(format!(
            "/sys/class/uio/uio{}/maps/map{}/{}",
            self.num, mapping, fname
        ))
        .await?;
        Ok(usize::from_str_radix(
            n.strip_prefix("0x")
                .ok_or_else(|| anyhow::anyhow!("prefix 0x not present"))?
                .trim_end(),
            16,
        )?)
    }

    /// Gives the size of a UIO mapping.
    ///
    /// The map size is obtained from the file
    /// `/sys/class/uio/uio*/maps/map*/size`.
    pub async fn map_size(&self, mapping: usize) -> Result<usize> {
        self.read_mapping_hex(mapping, "size").await
    }

    /// Gives the offset of a UIO mapping.
    ///
    /// The offset is obtained from the file
    /// `/sys/class/uio/uio*/maps/map*/offset`.
    pub async fn map_offset(&self, mapping: usize) -> Result<usize> {
        self.read_mapping_hex(mapping, "offset").await
    }

    /// Gives the address of a UIO mapping.
    ///
    /// The address is obtained from the file
    /// `/sys/class/uio/uio*/maps/map*/address`.
    pub async fn map_addr(&self, mapping: usize) -> Result<usize> {
        self.read_mapping_hex(mapping, "addr").await
    }

    /// Enables interrupts.
    ///
    /// This function enables the interrupts for an UIO device by writing a `1`
    /// to the corresponding character device.
    pub async fn irq_enable(&mut self) -> Result<()> {
        let bytes = 1u32.to_ne_bytes();
        self.file.write_all(&bytes).await?;
        Ok(())
    }

    /// Disables interrupts.
    ///
    /// This function disables the interrupts for an UIO device by writing a `0`
    /// to the corresponding character device.
    pub async fn irq_disable(&mut self) -> Result<()> {
        let bytes = 0u32.to_ne_bytes();
        self.file.write_all(&bytes).await?;
        Ok(())
    }

    /// Waits for an interrupt.
    ///
    /// This function waits for an interrupt from a UIO device by reading from
    /// the corresponding character device.
    pub async fn irq_wait(&mut self) -> Result<u32> {
        let mut bytes = [0; 4];
        self.file.read_exact(&mut bytes).await?;
        Ok(u32::from_ne_bytes(bytes))
    }
}

impl Mapping {
    /// Gives the virtual address of the mapping.
    ///
    /// This function returns a pointer to the beginning of the virtual address
    /// space to which the device IO is mapped.
    pub fn addr(&self) -> *mut libc::c_void {
        self.effective
    }
}

/// Unmaps the UIO device mapping.
impl Drop for Mapping {
    fn drop(&mut self) {
        unsafe {
            // TODO: control failure
            libc::munmap(self.base, self.map_size);
        }
    }
}
