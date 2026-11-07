use iced::widget::{button, column, row, slider, text, Column};
use iced::{Alignment, Element, Length, Sandbox, Settings};
use std::fs;
use std::path::Path;
use std::process::Command;

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
                
                match fs::write(&self.config_path, new_config) {
                    Ok(_) => self.save_status = String::from("Saved successfully!"),
                    Err(e) => self.save_status = format!("Error saving: {}", e),
                }
            }
            Message::ApplyConfig => {
                self.apply_status = String::from("Applying...");
                
                // Execute the `intel-undervolt apply` shell command
                match Command::new("intel-undervolt").arg("apply").output() {
                    Ok(output) => {
                        if output.status.success() {
                            self.apply_status = String::from("Applied successfully!");
                        } else {
                            // Extract stderr if the command failed (e.g., not running as root)
                            let error_msg = String::from_utf8_lossy(&output.stderr);
                            self.apply_status = format!("Failed: {}", error_msg.trim());
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

        // Action buttons row
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
        
        // Status readouts
        col = col.push(text(&self.save_status).size(14));
        if !self.apply_status.is_empty() {
            col = col.push(text(&self.apply_status).size(14));
        }

        col.into()
    }
}

fn parse_config<P: AsRef<Path>>(path: P) -> Result<Vec<VoltageEntry>, std::io::Error> {
    let content = fs::read_to_string(path)?;
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
