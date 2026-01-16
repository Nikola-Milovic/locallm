use crate::clipboard;
use crate::config::Config;
use crate::gpu_stats::{read_amd_gpu_stats, GpuStats};
use crate::ollama::{ChatMessage, OllamaClient};
use iced::widget::{
    button, column, container, horizontal_space, pick_list, row, scrollable, text, text_editor,
    vertical_space, Column,
};
use iced::keyboard;
use iced::{Element, Length, Subscription, Task, Theme};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Message {
    // Input
    InputChanged(text_editor::Action),
    Submit,

    // Ollama
    ModelsLoaded(Result<Vec<String>, String>),
    ModelSelected(String),
    RefreshModels,
    OllamaStatus(bool),

    // Streaming response
    ResponseComplete(Result<String, String>),

    // Chat management
    ClearChat,
    CopyMessage(usize),
    CopyComplete(Result<(), String>),

    // GPU stats
    GpuStatsTick,
    GpuStatsUpdated(Option<GpuStats>),
    
    // Keyboard
    ShiftPressed,
    ShiftReleased,
}

#[derive(Debug, Clone)]
pub struct ChatEntry {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Status {
    Disconnected,
    Connected,
    Generating,
}

pub struct App {
    config: Config,
    client: OllamaClient,

    // Models
    available_models: Vec<String>,
    selected_model: Option<String>,

    // Chat state
    chat_history: Vec<ChatEntry>,
    input_content: text_editor::Content,
    status: Status,
    status_message: String,

    // GPU stats
    gpu_stats: Option<GpuStats>,
    
    // Track if shift is held
    shift_held: bool,
}

impl App {
    pub fn new(config: Config) -> (Self, Task<Message>) {
        let client = OllamaClient::new(&config.ollama_url);

        let app = Self {
            config,
            client: client.clone(),
            available_models: Vec::new(),
            selected_model: None,
            chat_history: Vec::new(),
            input_content: text_editor::Content::new(),
            status: Status::Disconnected,
            status_message: String::from("Connecting to Ollama..."),
            gpu_stats: None,
            shift_held: false,
        };

        // Initial tasks: check Ollama status and load models
        let check_task = Task::perform(
            async move { client.health_check().await.unwrap_or(false) },
            Message::OllamaStatus,
        );

        (app, check_task)
    }

    pub fn title(&self) -> String {
        String::from("LocalLM")
    }

    pub fn theme(&self) -> Theme {
        Theme::TokyoNightStorm
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let gpu_sub = if self.config.show_gpu_stats {
            iced::time::every(Duration::from_secs(2)).map(|_| Message::GpuStatsTick)
        } else {
            Subscription::none()
        };

        // Track shift key state
        let shift_sub = keyboard::on_key_press(|key, _| {
            match key {
                keyboard::Key::Named(keyboard::key::Named::Shift) => Some(Message::ShiftPressed),
                _ => None,
            }
        });
        
        let shift_release_sub = keyboard::on_key_release(|key, _| {
            match key {
                keyboard::Key::Named(keyboard::key::Named::Shift) => Some(Message::ShiftReleased),
                _ => None,
            }
        });

        Subscription::batch([gpu_sub, shift_sub, shift_release_sub])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InputChanged(action) => {
                // Check if this is an Enter key - submit if Shift is NOT held
                let is_enter = matches!(
                    action,
                    text_editor::Action::Edit(text_editor::Edit::Enter)
                );
                
                if is_enter && !self.shift_held {
                    // Submit instead of inserting newline
                    return self.update(Message::Submit);
                }
                
                self.input_content.perform(action);
                Task::none()
            }

            Message::Submit => {
                let input_text = self.input_content.text();
                if input_text.trim().is_empty() {
                    return Task::none();
                }
                if self.status == Status::Generating {
                    return Task::none();
                }

                let Some(model) = self.selected_model.clone() else {
                    self.status_message = String::from("No model selected");
                    return Task::none();
                };

                // Add user message to history
                let user_msg = input_text.trim().to_string();
                self.chat_history.push(ChatEntry {
                    role: "user".to_string(),
                    content: user_msg.clone(),
                });
                self.input_content = text_editor::Content::new();
                self.status = Status::Generating;
                self.status_message = String::from("Generating...");

                // Build messages for API
                let mut messages: Vec<ChatMessage> = Vec::new();

                // Add system prompt if configured
                if let Some(ref sys) = self.config.system_prompt {
                    messages.push(ChatMessage {
                        role: "system".to_string(),
                        content: sys.clone(),
                    });
                }

                // Add chat history
                for entry in &self.chat_history {
                    messages.push(ChatMessage {
                        role: entry.role.clone(),
                        content: entry.content.clone(),
                    });
                }

                let client = self.client.clone();
                Task::perform(
                    async move {
                        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

                        // Spawn the streaming request
                        let handle = tokio::spawn(async move {
                            client.chat_stream(&model, messages, tx).await
                        });

                        // Collect all tokens
                        let mut full_response = String::new();
                        while let Some(token) = rx.recv().await {
                            full_response.push_str(&token);
                        }

                        // Wait for completion
                        match handle.await {
                            Ok(Ok(_)) => Ok(full_response),
                            Ok(Err(e)) => Err(e.to_string()),
                            Err(e) => Err(e.to_string()),
                        }
                    },
                    Message::ResponseComplete,
                )
            }

            Message::ResponseComplete(result) => {
                self.status = Status::Connected;
                match result {
                    Ok(response) => {
                        if !response.is_empty() {
                            self.chat_history.push(ChatEntry {
                                role: "assistant".to_string(),
                                content: response,
                            });
                        }
                        self.status_message = String::from("Ready");
                    }
                    Err(e) => {
                        self.status_message = format!("Error: {e}");
                    }
                }
                Task::none()
            }

            Message::OllamaStatus(connected) => {
                if connected {
                    self.status = Status::Connected;
                    self.status_message = String::from("Connected to Ollama");

                    // Load models
                    let client = self.client.clone();
                    Task::perform(
                        async move {
                            client
                                .list_models()
                                .await
                                .map(|models| {
                                    models.into_iter().map(|m| m.name).collect::<Vec<String>>()
                                })
                                .map_err(|e| e.to_string())
                        },
                        Message::ModelsLoaded,
                    )
                } else {
                    self.status = Status::Disconnected;
                    self.status_message = String::from("Ollama not running");
                    Task::none()
                }
            }

            Message::ModelsLoaded(result) => {
                match result {
                    Ok(models) => {
                        self.available_models = models;

                        // Select default model or first available
                        if self.selected_model.is_none() {
                            self.selected_model = self
                                .config
                                .default_model
                                .clone()
                                .filter(|m| self.available_models.contains(m))
                                .or_else(|| self.available_models.first().cloned());
                        }

                        if self.available_models.is_empty() {
                            self.status_message = String::from("No models found. Run: ollama pull <model>");
                        } else {
                            self.status_message = format!("{} models available", self.available_models.len());
                        }
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to load models: {e}");
                    }
                }
                Task::none()
            }

            Message::ModelSelected(model) => {
                self.selected_model = Some(model);
                Task::none()
            }

            Message::RefreshModels => {
                let client = self.client.clone();
                Task::perform(
                    async move {
                        client
                            .list_models()
                            .await
                            .map(|models| {
                                models.into_iter().map(|m| m.name).collect::<Vec<String>>()
                            })
                            .map_err(|e| e.to_string())
                    },
                    Message::ModelsLoaded,
                )
            }

            Message::ClearChat => {
                self.chat_history.clear();
                self.input_content = text_editor::Content::new();
                self.status_message = String::from("Chat cleared");
                Task::none()
            }

            Message::CopyMessage(idx) => {
                if let Some(entry) = self.chat_history.get(idx) {
                    let content = entry.content.clone();
                    let role = entry.role.clone();
                    self.status_message = format!("📋 Copied {} message!", role);
                    Task::perform(
                        async move { clipboard::copy_to_clipboard(&content).await },
                        Message::CopyComplete,
                    )
                } else {
                    Task::none()
                }
            }

            Message::CopyComplete(result) => {
                if let Err(e) = result {
                    self.status_message = format!("Copy failed: {e}");
                }
                // On success, keep the message we already set
                Task::none()
            }

            Message::GpuStatsTick => {
                Task::perform(async { read_amd_gpu_stats().await }, Message::GpuStatsUpdated)
            }

            Message::GpuStatsUpdated(stats) => {
                self.gpu_stats = stats;
                Task::none()
            }
            
            Message::ShiftPressed => {
                self.shift_held = true;
                Task::none()
            }
            
            Message::ShiftReleased => {
                self.shift_held = false;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        // Model selector row
        let model_picker = pick_list(
            self.available_models.clone(),
            self.selected_model.clone(),
            Message::ModelSelected,
        )
        .placeholder("Select model...")
        .width(Length::FillPortion(3));

        let refresh_btn = button("↻").on_press(Message::RefreshModels);
        let clear_btn = button("Clear").on_press(Message::ClearChat);

        let toolbar = row![
            model_picker,
            refresh_btn,
            clear_btn,
            horizontal_space(),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        // Chat history
        let chat_content: Element<Message> = if self.chat_history.is_empty() && self.status != Status::Generating {
            container(
                text("Start a conversation...")
                    .size(16)
                    .color(iced::Color::from_rgb(0.5, 0.5, 0.5)),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else {
            let mut chat_column = Column::new().spacing(12).padding(8);

            for (idx, entry) in self.chat_history.iter().enumerate() {
                let bubble = self.render_message(idx, &entry.role, &entry.content);
                chat_column = chat_column.push(bubble);
            }

            // Show "thinking" indicator while generating
            if self.status == Status::Generating {
                let thinking = container(text("...").size(14))
                    .padding(12)
                    .style(container::bordered_box)
                    .max_width(500);
                chat_column = chat_column.push(
                    row![thinking, horizontal_space()].width(Length::Fill)
                );
            }

            scrollable(chat_column)
                .height(Length::FillPortion(5))
                .into()
        };

        // Input area
        let is_generating = self.status == Status::Generating;
        let input = text_editor(&self.input_content)
            .placeholder("Type your message...")
            .on_action(Message::InputChanged)
            .height(Length::Fixed(80.0));

        let send_btn = button(if is_generating { "..." } else { "Send" })
            .on_press_maybe((!is_generating && self.selected_model.is_some()).then_some(Message::Submit));

        let input_row = row![input, send_btn].spacing(8).align_y(iced::Alignment::End);

        // Status bar with GPU stats
        let status_text = text(&self.status_message).size(12);

        let gpu_text = if let Some(ref stats) = self.gpu_stats {
            text(format!(
                "VRAM: {}/{}MB ({:.0}%) | GPU: {}%{}",
                stats.vram_used_mb,
                stats.vram_total_mb,
                stats.vram_usage_percent(),
                stats.gpu_usage_percent,
                stats.temperature_c.map(|t| format!(" | {}°C", t)).unwrap_or_default()
            ))
            .size(12)
        } else {
            text("").size(12)
        };

        let status_bar = row![status_text, horizontal_space(), gpu_text]
            .spacing(16)
            .align_y(iced::Alignment::Center);

        // Main layout
        let content = column![
            toolbar,
            vertical_space().height(8),
            chat_content,
            vertical_space().height(8),
            input_row,
            vertical_space().height(4),
            status_bar,
        ]
        .padding(16)
        .spacing(4);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn render_message(&self, idx: usize, role: &str, content: &str) -> Element<'_, Message> {
        let is_user = role == "user";

        let msg_text = text(content.to_string()).size(14);

        // Make the bubble a clickable button to copy
        let bubble = button(
            container(msg_text)
                .padding(12)
                .max_width(500)
        )
        .style(if is_user {
            button::secondary
        } else {
            button::primary
        })
        .on_press(Message::CopyMessage(idx));

        if is_user {
            row![horizontal_space(), bubble]
                .width(Length::Fill)
                .into()
        } else {
            row![bubble, horizontal_space()]
                .width(Length::Fill)
                .into()
        }
    }
}
