use crate::models::update::UpdateSource;
use serde_json::json;
pub fn support_info() -> serde_json::Value {
    let info = os_info::get();
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "runtime": "Windows 桌面端",
        "os": format!("{} {}", info.os_type(), info.version()),
        "arch": std::env::consts::ARCH,
    })
}

pub fn open_project_repository(source: UpdateSource) -> Result<(), String> {
    let address = match source {
        UpdateSource::Gitee => "https://gitee.com/liuliang6666/prompt-draft",
        UpdateSource::Github => "https://github.com/zorvilo/prompt-draft",
    };
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("explorer.exe")
            .arg(address)
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|_| "无法打开浏览器，请稍后重试。".to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = address;
        Err("当前平台暂不支持打开仓库。".into())
    }
}

#[cfg(test)]
mod support_info_tests {
    #[test]
    fn support_information_contains_only_public_environment_fields() {
        let value = super::support_info();
        let object = value.as_object().unwrap();
        assert_eq!(object.len(), 4);
        for field in ["version", "runtime", "os", "arch"] {
            assert!(!object[field].as_str().unwrap().is_empty());
        }
        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["arch"], std::env::consts::ARCH);
    }
}
