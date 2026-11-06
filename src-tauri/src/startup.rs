#[cfg(windows)]
mod windows_startup {
    use std::{os::windows::ffi::OsStrExt, path::Path};
    use windows::{
        core::{w, HSTRING, PCWSTR},
        Win32::{
            Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
            System::Registry::*,
        },
    };

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const APPROVAL_KEY: &str =
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

    struct Key(HKEY);

    impl Drop for Key {
        fn drop(&mut self) {
            let _ = unsafe { RegCloseKey(self.0) };
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Value(REG_VALUE_TYPE, Vec<u8>);

    fn open(path: &str, create: bool) -> Result<Option<Key>, String> {
        let mut key = HKEY::default();
        let path = HSTRING::from(path);
        let result = unsafe {
            if create {
                RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    &path,
                    None,
                    PCWSTR::null(),
                    REG_OPTION_NON_VOLATILE,
                    KEY_QUERY_VALUE | KEY_SET_VALUE,
                    None,
                    &mut key,
                    None,
                )
            } else {
                RegOpenKeyExW(HKEY_CURRENT_USER, &path, None, KEY_QUERY_VALUE, &mut key)
            }
        };
        if result == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        result
            .ok()
            .map_err(|error| format!("无法访问开机启动注册表：{error}"))?;
        Ok(Some(Key(key)))
    }

    fn read(path: &str) -> Result<Option<Value>, String> {
        let Some(key) = open(path, false)? else {
            return Ok(None);
        };
        let mut kind = REG_VALUE_TYPE::default();
        let mut size = 0;
        let result = unsafe {
            RegQueryValueExW(
                key.0,
                w!("CoworkPal"),
                None,
                Some(&mut kind),
                None,
                Some(&mut size),
            )
        };
        if result == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        result
            .ok()
            .map_err(|error| format!("无法读取开机启动注册表：{error}"))?;
        let mut bytes = vec![0; size as usize];
        unsafe {
            RegQueryValueExW(
                key.0,
                w!("CoworkPal"),
                None,
                Some(&mut kind),
                Some(bytes.as_mut_ptr()),
                Some(&mut size),
            )
        }
        .ok()
        .map_err(|error| format!("无法读取开机启动注册表：{error}"))?;
        bytes.truncate(size as usize);
        Ok(Some(Value(kind, bytes)))
    }

    fn write(path: &str, value: Option<&Value>) -> Result<(), String> {
        // A missing value is the only deletion error that is safe to ignore.
        if value.is_none() && read(path)?.is_none() {
            return Ok(());
        }
        let key = open(path, true)?.ok_or("无法创建开机启动注册表项")?;
        let result = unsafe {
            match value {
                Some(Value(kind, bytes)) => {
                    RegSetValueExW(key.0, w!("CoworkPal"), None, *kind, Some(bytes))
                }
                None => RegDeleteValueW(key.0, w!("CoworkPal")),
            }
        };
        if result == ERROR_SUCCESS || (value.is_none() && result == ERROR_FILE_NOT_FOUND) {
            return Ok(());
        }
        Err(format!("无法更新开机启动注册表：{}", result.to_hresult()))
    }

    fn command_value(exe: &Path) -> Result<Value, String> {
        let mut command: Vec<u16> = std::iter::once(b'"' as u16)
            .chain(exe.as_os_str().encode_wide())
            .chain(std::iter::once(b'"' as u16))
            .collect();
        if command.len() > 260 {
            return Err(
                "安装路径过长，Windows 无法通过 Run 项自启，请安装到较短的路径".to_string(),
            );
        }
        command.push(0);
        Ok(Value(
            REG_SZ,
            command.into_iter().flat_map(u16::to_le_bytes).collect(),
        ))
    }

    pub(crate) struct Registration {
        run: Option<Value>,
        approval: Option<Value>,
    }

    impl Registration {
        pub(crate) fn capture() -> Result<Self, String> {
            Self::capture_at(RUN_KEY, APPROVAL_KEY)
        }

        fn capture_at(run: &str, approval: &str) -> Result<Self, String> {
            Ok(Self {
                run: read(run)?,
                approval: read(approval)?,
            })
        }

        pub(crate) fn restore(&self) -> Result<(), String> {
            self.restore_at(RUN_KEY, APPROVAL_KEY)
        }

        fn restore_at(&self, run: &str, approval: &str) -> Result<(), String> {
            write(run, self.run.as_ref())?;
            write(approval, self.approval.as_ref())
        }
    }

    fn sync_at(
        run: &str,
        approval: &str,
        exe: &Path,
        enabled: bool,
        explicit: bool,
    ) -> Result<(), String> {
        let previous = Registration::capture_at(run, approval)?;
        let target = enabled.then(|| command_value(exe)).transpose()?;
        let result = (|| {
            write(run, target.as_ref())?;
            // Reset Windows' disabled marker only when the user explicitly enables startup.
            if enabled && explicit {
                write(approval, None)?;
            }
            if read(run)? != target {
                return Err("开机启动注册表回读校验失败".to_string());
            }
            Ok(())
        })();
        if let Err(error) = result {
            return match previous.restore_at(run, approval) {
                Ok(()) => Err(error),
                Err(rollback) => Err(format!("{error}；回退开机启动失败：{rollback}")),
            };
        }
        Ok(())
    }

    pub(crate) fn sync(enabled: bool, explicit: bool) -> Result<(), String> {
        let exe = std::env::current_exe().map_err(|error| error.to_string())?;
        sync_at(RUN_KEY, APPROVAL_KEY, &exe, enabled, explicit)
    }

    pub(crate) fn is_enabled() -> Result<bool, String> {
        let Some(value) = read(RUN_KEY)? else {
            return Ok(false);
        };
        let exe = std::env::current_exe().map_err(|error| error.to_string())?;
        let Ok(expected) = command_value(&exe) else {
            return Ok(false);
        };
        if value != expected {
            return Ok(false);
        }
        Ok(!read(APPROVAL_KEY)?.is_some_and(|Value(kind, bytes)| {
            kind == REG_BINARY && matches!(bytes.first(), Some(3 | 7))
        }))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn startup_roundtrip_repairs_paths_and_respects_windows_disable() {
            let root = format!(r"Software\CoworkPal.Tests\Startup-{}", std::process::id());
            let run = format!(r"{root}\Run");
            let approval = format!(r"{root}\Approval");
            let exe = Path::new(r"C:\Program Files\中文目录\CoworkPal\cowork-pal.exe");
            sync_at(&run, &approval, exe, true, true).unwrap();
            assert_eq!(read(&run).unwrap(), Some(command_value(exe).unwrap()));
            let disabled = Value(REG_BINARY, vec![3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            write(&approval, Some(&disabled)).unwrap();
            let previous = Registration::capture_at(&run, &approval).unwrap();
            let moved = Path::new(r"D:\Apps\CoworkPal\cowork-pal.exe");
            sync_at(&run, &approval, moved, true, false).unwrap();
            assert_eq!(read(&run).unwrap(), Some(command_value(moved).unwrap()));
            assert_eq!(read(&approval).unwrap(), Some(disabled.clone()));
            sync_at(&run, &approval, moved, true, true).unwrap();
            assert_eq!(read(&approval).unwrap(), None);
            previous.restore_at(&run, &approval).unwrap();
            assert_eq!(read(&run).unwrap(), Some(command_value(exe).unwrap()));
            assert_eq!(read(&approval).unwrap(), Some(disabled));
            sync_at(&run, &approval, exe, false, true).unwrap();
            sync_at(&run, &approval, exe, false, true).unwrap();
            assert_eq!(read(&run).unwrap(), None);
            assert!(command_value(Path::new(&format!(
                "C:\\{}\\cowork-pal.exe",
                "a".repeat(260)
            )))
            .is_err());
            unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, &HSTRING::from(root)) }
                .ok()
                .unwrap();
        }
    }
}

#[cfg(windows)]
pub(crate) use windows_startup::{is_enabled, sync, Registration};

#[cfg(not(windows))]
pub(crate) fn sync(_enabled: bool, _explicit: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub(crate) fn is_enabled() -> Result<bool, String> {
    Ok(false)
}
