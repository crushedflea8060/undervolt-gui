use iced::widget::{button, column, row, slider, text, Column};
use iced::{Alignment, Element, Length, Sandbox, Settings};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn main() -> iced::Result {
    IntelUndervoltApp::run(Settings::default())
}

#[derive(Debug, Clone)]
struct VoltageEntry {
    id: usize,
    name: String,
    value: f32,
}

struct IntelUndervoltApp {
    entries: Vec<VoltageEntry>,
    config_path: String,
    save_status: String,
    apply_status: String,
}

#[derive(Debug, Clone)]
enum Message {
    VoltageChanged(usize, f32),
    SaveConfig,
    ApplyConfig,
}

impl Sandbox for IntelUndervoltApp {
    type Message = Message;

    fn new() -> Self {
        let config_path = "/etc/intel-undervolt.conf".to_string();
        
        let entries = parse_config(&config_path).unwrap_or_else(|_| vec![
            VoltageEntry { id: 0, name: "CPU".to_string(), value: 0.0 },
            VoltageEntry { id: 1, name: "GPU".to_string(), value: 0.0 },
            VoltageEntry { id: 2, name: "CPU Cache".to_string(), value: 0.0 },
            VoltageEntry { id: 3, name: "System Agent".to_string(), value: 0.0 },
            VoltageEntry { id: 4, name: "Analog I/O".to_string(), value: 0.0 },
        ]);

        Self {
            entries,
            config_path,
            save_status: String::from("Ready"),
            apply_status: String::new(),
        }
    }

    fn title(&self) -> String {
        String::from("Intel Undervolt Configuration")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::VoltageChanged(id, value) => {
                if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
                    entry.value = value;
                    self.save_status = String::from("Unsaved changes");
                    self.apply_status = String::new(); // Clear previous apply status
                }
            }
            Message::SaveConfig => {
                let mut new_config = String::new();
                for entry in &self.entries {
                    new_config.push_str(&format!(
                        "undervolt {} '{}' {:.2}\n",
                        entry.id, entry.name, entry.value
                    ));
                }
                
                // Use `pkexec tee` to safely pipe the new config to the privileged file
                match Command::new("pkexec")
                    .arg("tee")
                    .arg(&self.config_path)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null()) // We don't need tee's stdout
                    .spawn()
                {
                    Ok(mut child) => {
                        if let Some(mut stdin) = child.stdin.take() {
                            let _ = stdin.write_all(new_config.as_bytes());
                        }
                        
                        match child.wait() {
                            Ok(status) if status.success() => {
                                self.save_status = String::from("Saved successfully!");
                            }
                            Ok(_) => {
                                self.save_status = String::from("Authentication cancelled or failed.");
                            }
                            Err(e) => {
                                self.save_status = format!("Error waiting on process: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        self.save_status = format!("Failed to launch pkexec: {}", e);
                    }
                }
            }
            Message::ApplyConfig => {
                self.apply_status = String::from("Applying...");
                
                // Execute `pkexec intel-undervolt apply` for graphical sudo prompt
                match Command::new("pkexec")
                    .arg("intel-undervolt")
                    .arg("apply")
                    .output() 
                {
                    Ok(output) => {
                        if output.status.success() {
                            self.apply_status = String::from("Applied successfully!");
                        } else {
                            // Extract stderr if the command failed or user cancelled auth
                            let error_msg = String::from_utf8_lossy(&output.stderr);
                            self.apply_status = format!("Failed/Cancelled: {}", error_msg.trim());
                        }
                    }
                    Err(e) => {
                        self.apply_status = format!("Execution error: {}", e);
                    }
                }
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let mut col = Column::new()
            .padding(30)
            .spacing(20)
            .align_items(Alignment::Center);

        col = col.push(text("Intel Undervolt Values").size(24));

        for entry in &self.entries {
            let slider_row = row![
                text(&entry.name).width(Length::Fixed(120.0)),
                slider(-250.0..=0.0, entry.value, move |v| Message::VoltageChanged(
                    entry.id, v
                ))
                .step(0.5)
                .width(Length::Fill),
                text(format!("{:.2} mV", entry.value)).width(Length::Fixed(80.0)),
            ]
            .spacing(15)
            .align_items(Alignment::Center);

            col = col.push(slider_row);
        }

        let action_row = row![
            button(text("Save to Config File"))
                .padding(10)
                .on_press(Message::SaveConfig),
            button(text("Apply Config"))
                .padding(10)
                .on_press(Message::ApplyConfig),
        ]
        .spacing(20)
        .align_items(Alignment::Center);

        col = col.push(action_row);
        
        col = col.push(text(&self.save_status).size(14));
        if !self.apply_status.is_empty() {
            col = col.push(text(&self.apply_status).size(14));
        }

        col.into()
    }
}

fn parse_config<P: AsRef<Path>>(path: P) -> Result<Vec<VoltageEntry>, std::io::Error> {
    // Read the file securely using `pkexec cat`
    let output = Command::new("pkexec")
        .arg("cat")
        .arg(path.as_ref().as_os_str())
        .output()?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("Authentication required or failed: {}", err_msg.trim())
        ));
    }

    let content = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("undervolt ") {
            let start_quote = line.find('\'');
            let end_quote = line.rfind('\'');

            if let (Some(start), Some(end)) = (start_quote, end_quote) {
                if start < end {
                    let name = line[start + 1..end].to_string();
                    let prefix_parts: Vec<&str> = line[..start].split_whitespace().collect();
                    let suffix = line[end + 1..].trim();

                    if let (Some(id_str), Ok(value)) = (prefix_parts.get(1), suffix.parse::<f32>()) {
                        if let Ok(id) = id_str.parse::<usize>() {
                            entries.push(VoltageEntry { id, name, value });
                        }
                    }
                }
            }
        }
    }
    
    entries.sort_by_key(|e| e.id);
    Ok(entries)
}
