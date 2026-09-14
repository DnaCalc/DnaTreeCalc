//! Machine identity and process memory probes for the bench artifact.
//!
//! Everything here is a runtime probe of the host the bench ran on: the
//! standard library's constants, environment variables, `rustc -vV`, and —
//! on Windows — one PowerShell CIM query for the CPU name and physical
//! memory, plus one `Get-Process` query for the process's peak working set.
//! The workspace forbids `unsafe_code`, so no Win32 call is made directly
//! and no counting `GlobalAlloc` exists; a probe that fails leaves its field
//! `None` and names itself in `probe_failures`, never a fabricated value.

use std::process::Command;

use crate::bench_report::MachineIdentity;

/// Probe the machine the bench is running on.
#[must_use]
pub fn probe_machine() -> MachineIdentity {
    let mut probe_failures = Vec::new();
    let logical_cpus = match std::thread::available_parallelism() {
        Ok(n) => Some(n.get() as u64),
        Err(error) => {
            probe_failures.push(format!("available_parallelism: {error}"));
            None
        }
    };
    let processor_identifier = std::env::var("PROCESSOR_IDENTIFIER").ok();
    let rustc_version = match rustc_vv() {
        Ok(text) => Some(text),
        Err(error) => {
            probe_failures.push(format!("rustc -vV: {error}"));
            None
        }
    };
    let (cpu_name, total_physical_memory_bytes) = match windows_cpu_and_memory() {
        Ok(pair) => pair,
        Err(error) => {
            probe_failures.push(format!("cpu/memory probe: {error}"));
            (None, None)
        }
    };
    MachineIdentity {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        family: std::env::consts::FAMILY.to_string(),
        logical_cpus,
        processor_identifier,
        cpu_name,
        total_physical_memory_bytes,
        rustc_version,
        debug_assertions: cfg!(debug_assertions),
        probe_failures,
    }
}

/// `rustc -vV`, the lines joined by `; `.
fn rustc_vv() -> Result<String, String> {
    let output = Command::new("rustc")
        .arg("-vV")
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!("exit status {}", output.status));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("; "))
}

/// Windows only: `Win32_Processor.Name` and
/// `Win32_ComputerSystem.TotalPhysicalMemory` through PowerShell CIM. On
/// any other platform both are `None` with a named failure.
fn windows_cpu_and_memory() -> Result<(Option<String>, Option<u64>), String> {
    if !cfg!(windows) {
        return Err("only probed on Windows".to_string());
    }
    let script = "(Get-CimInstance Win32_Processor | Select-Object -First 1).Name; \
                  (Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory";
    let text = powershell(script)?;
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let cpu_name = lines.next().map(str::to_string);
    let memory = lines.next().and_then(|line| line.parse::<u64>().ok());
    Ok((cpu_name, memory))
}

/// The current process's peak working set in bytes (Windows, via
/// `Get-Process`), or the reason it is unavailable. Sampled once, at the
/// end of a run, so it covers everything the process did.
pub fn peak_working_set_bytes() -> Result<u64, String> {
    if !cfg!(windows) {
        return Err("only probed on Windows (Get-Process PeakWorkingSet64)".to_string());
    }
    let pid = std::process::id();
    let text = powershell(&format!("(Get-Process -Id {pid}).PeakWorkingSet64"))?;
    text.trim()
        .parse::<u64>()
        .map_err(|error| format!("PeakWorkingSet64 parse: {error} ({text:?})"))
}

fn powershell(script: &str) -> Result<String, String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "powershell exit status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The probe never panics and always names its platform; whatever could
    /// not be probed is listed, not silently absent.
    #[test]
    fn probe_machine_names_the_platform_and_lists_failures() {
        let machine = probe_machine();
        assert_eq!(machine.os, std::env::consts::OS);
        assert_eq!(machine.arch, std::env::consts::ARCH);
        assert_eq!(machine.debug_assertions, cfg!(debug_assertions));
        if machine.cpu_name.is_none() || machine.total_physical_memory_bytes.is_none() {
            assert!(
                machine
                    .probe_failures
                    .iter()
                    .any(|failure| failure.starts_with("cpu/memory probe"))
                    || cfg!(windows),
                "an absent CPU/memory field is either a named failure or a Windows CIM row that \
                 came back empty: {machine:#?}"
            );
        }
        if machine.rustc_version.is_none() {
            assert!(
                machine
                    .probe_failures
                    .iter()
                    .any(|failure| failure.starts_with("rustc -vV"))
            );
        }
    }
}
