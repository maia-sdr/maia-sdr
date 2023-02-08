//! Userspace driver for the rxbuffer device of the maia-sdr kernel module.
//!
//! This implements memory mapping and cache invalidation control, as provided
//! by the rxbuffer device implemented in `maia-sdr.ko`. The rxbuffer device
//! gives access to a DMA buffer formed by a ring of buffers of equal size. The
//! cache of each of the buffers in the ring can be invalidated independently.

use anyhow::Result;
use std::os::unix::io::{AsRawFd, RawFd};
use tokio::fs;

/// Receive DMA buffer.
///
/// This struct corresponds to a rxbuffer device.
#[derive(Debug)]
pub struct RxBuffer {
    _file: fs::File,
    fd: RawFd,
    buffer: *mut libc::c_void,
    buffer_size: usize,
    num_buffers: usize,
}

unsafe impl Send for RxBuffer {}

impl RxBuffer {
    /// Opens an rxbuffer device.
    ///
    /// The name of the device corresponds to the filename of the character
    /// device in `/dev`.
    pub async fn new(name: &str) -> Result<RxBuffer> {
        let file = fs::File::open(format!("/dev/{name}")).await?;
        let fd = file.as_raw_fd();
        let buffer_size = usize::from_str_radix(
            fs::read_to_string(format!("/sys/class/maia-sdr/{name}/device/buffer_size"))
                .await?
                .trim_end()
                .trim_start_matches("0x"),
            16,
        )?;
        let num_buffers =
            fs::read_to_string(format!("/sys/class/maia-sdr/{name}/device/num_buffers"))
                .await?
                .trim_end()
                .parse::<usize>()?;
        let buffer = unsafe {
            match libc::mmap(
                std::ptr::null_mut::<libc::c_void>(),
                buffer_size * num_buffers,
                libc::PROT_READ,
                libc::MAP_SHARED,
                fd,
                0,
            ) {
                libc::MAP_FAILED => anyhow::bail!("mmap rxbuffer failed"),
                x => x,
            }
        };
        Ok(RxBuffer {
            _file: file,
            fd,
            buffer,
            buffer_size,
            num_buffers,
        })
    }

    /// Returns the size in bytes of each of the buffers in the ring.
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    /// Returns the number of buffers in the ring.
    pub fn num_buffers(&self) -> usize {
        self.num_buffers
    }

    /// Returns a slice that contains one of the buffers in the ring.
    ///
    /// # Panics
    ///
    /// This function panics if `num_buffer` is greater or equal to the number
    /// of buffers in the ring.
    pub fn buffer_as_slice(&self, num_buffer: usize) -> &[u8] {
        assert!(num_buffer < self.num_buffers);
        unsafe {
            std::slice::from_raw_parts(
                self.buffer.add(num_buffer * self.buffer_size) as *const u8,
                self.buffer_size,
            )
        }
    }

    /// Invalidates the cache of one of the buffers in the ring.
    ///
    /// After calling this function, the contents of the CPU caches
    /// corresponding to the buffer have been invalidated, so changes in the
    /// buffer produced by non-coherent writes done by the FPGA can be observed
    /// by the CPU.
    ///
    /// This function should be called before reading data from the buffer,
    /// unless we know that the FPGA has not written to that buffer since the
    /// last time that we invalidated its corresponding caches.
    pub fn cache_invalidate(&self, num_buffer: usize) -> Result<()> {
        assert!(num_buffer < self.num_buffers);
        unsafe { ioctl::maia_kmod_cacheinv(self.fd, num_buffer as _) }?;
        Ok(())
    }
}

mod ioctl {
    use nix::ioctl_write_int;

    const MAIA_SDR_IOC_MAGIC: u8 = b'M';
    const MAIA_SDR_CACHEINV: u8 = 0;

    ioctl_write_int!(maia_kmod_cacheinv, MAIA_SDR_IOC_MAGIC, MAIA_SDR_CACHEINV);
}

impl Drop for RxBuffer {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.buffer, self.buffer_size * self.num_buffers);
        }
    }
}
