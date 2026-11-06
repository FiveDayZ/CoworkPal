#[cfg(windows)]
fn notifier(
    app: &tauri::AppHandle,
) -> windows::core::Result<windows::UI::Notifications::ToastNotifier> {
    unsafe {
        windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(
            &windows::core::HSTRING::from(&app.config().identifier),
        )?;
    }
    windows::UI::Notifications::ToastNotificationManager::CreateToastNotifierWithId(
        &windows::core::HSTRING::from(&app.config().identifier),
    )
}

pub(crate) fn check_permission(_app: &tauri::AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::UI::Notifications::NotificationSetting;
        let setting = notifier(_app)
            .and_then(|notifier| {
                match notifier.Setting() {
                    // Windows creates a new desktop app's notification settings on its first toast.
                    Err(error) if error.code().0 == 0x80070490_u32 as i32 => {
                        notifier.Show(&toast(
                            "CoworkPal",
                            "通知已开启，硬件状态提醒将在这里显示。",
                        )?)?;
                        notifier.Setting()
                    }
                    result => result,
                }
            })
            .map_err(|error| format!("无法检查 Windows 通知权限：{error}"))?;
        if setting != NotificationSetting::Enabled {
            return Err("Windows 已禁用 CoworkPal 通知，请在系统设置 → 通知中开启".to_string());
        }
    }
    Ok(())
}

#[cfg(windows)]
fn toast(
    title: &str,
    body: &str,
) -> windows::core::Result<windows::UI::Notifications::ToastNotification> {
    use windows::{
        core::HSTRING,
        UI::Notifications::{ToastNotification, ToastNotificationManager, ToastTemplateType},
    };
    let content = ToastNotificationManager::GetTemplateContent(ToastTemplateType::ToastText02)?;
    let nodes = content.GetElementsByTagName(&HSTRING::from("text"))?;
    for (index, text) in [title, body].iter().enumerate() {
        nodes
            .Item(index as u32)?
            .AppendChild(&content.CreateTextNode(&HSTRING::from(*text))?)?;
    }
    ToastNotification::CreateToastNotification(&content)
}

pub(crate) fn show(app: &tauri::AppHandle, title: &str, body: &str) {
    #[cfg(windows)]
    if let Err(error) = toast(title, body).and_then(|toast| notifier(app)?.Show(&toast)) {
        tracing::warn!("failed to show Windows notification: {error}");
    }
    #[cfg(not(windows))]
    let _ = (app, title, body);
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn notification_text_is_preserved_without_xml_injection() {
        use windows::core::HSTRING;
        let toast = super::toast("CoCat <测试>", "CPU > 80% & \"警告\"").unwrap();
        let nodes = toast
            .Content()
            .unwrap()
            .GetElementsByTagName(&HSTRING::from("text"))
            .unwrap();
        assert_eq!(nodes.Item(0).unwrap().InnerText().unwrap(), "CoCat <测试>");
        assert_eq!(
            nodes.Item(1).unwrap().InnerText().unwrap(),
            "CPU > 80% & \"警告\""
        );
    }

    #[test]
    #[ignore = "sends one visible Windows notification"]
    fn desktop_notification_delivery() {
        use windows::{core::HSTRING, UI::Notifications::ToastNotificationManager};
        let id = HSTRING::from("com.coworkpal.desktop");
        unsafe { windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(&id) }.unwrap();
        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&id).unwrap();
        println!(
            "notification setting before delivery: {:?}",
            notifier.Setting()
        );
        notifier
            .Show(&super::toast("CoworkPal 设置验证", "Windows 系统通知通道测试").unwrap())
            .unwrap();
        assert_eq!(
            notifier.Setting().unwrap(),
            windows::UI::Notifications::NotificationSetting::Enabled
        );
    }
}
