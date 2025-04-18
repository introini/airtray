// SPDX-License-Identifier: GPL-3.0-only

use std::process::{Child, Command, Stdio};
use cosmic::app::{Core, Task};
use cosmic::iced::window::Id;
use cosmic::iced::Limits;
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::widget::{self, settings};
use cosmic::{Application, Element};

use crate::fl;

#[derive(Default)]
struct Uxplay {
    airplay: bool,
    process: Option<Child>,
}

impl Uxplay {
    fn new() -> Self {
        Self {
            airplay: false,
            process: None,
        }
    }

    /// Manages the UXPlay process based on the airplay setting.
    /// Spawns a new process if airplay is true and no process is running.
    /// Kills the existing process if airplay is false and a process is running.
    fn manage_uxplay_process(&mut self) -> Result<(), std::io::Error> {
        if self.airplay {
            // Only spawn a new process if we don't already have one running
            if self.process.is_none() {
                println!("Starting UXPlay process");
                let child = Command::new("uxplay")
                    .stdout(Stdio::piped())
                    .spawn()?;

                self.process = Some(child);
            }
        } else {
            // Kill the process if it exists
            if let Some(mut child) = self.process.take() {
                println!("Stopping UXPlay process");

                // Try to kill the process gracefully
                if let Err(e) = child.kill() {
                    println!("Failed to kill UXPlay process: {}", e);

                    // Even if kill fails, try to wait for it to avoid zombies
                    if let Err(e) = child.wait() {
                        println!("Failed to wait for UXPlay process: {}", e);
                    }
                } else {
                    // Wait for the process to exit
                    if let Err(e) = child.wait() {
                        println!("Failed to wait for UXPlay process: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Updates the airplay setting and manages the UXPlay process accordingly
    fn set_airplay(&mut self, enabled: bool) -> Result<(), std::io::Error> {
        // Only take action if the value is changing
        if self.airplay != enabled {
            self.airplay = enabled;
            self.manage_uxplay_process()?;
        }

        Ok(())
    }
}


/// This is the struct that represents your application.
/// It is used to define the data that will be used by your application.
#[derive(Default)]
pub struct AirTray {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    /// The popup id.
    popup: Option<Id>,
    /// Airplay toggler.
    airplay_toggle: bool,
    uxplay_process: Uxplay,
}

/// This is the enum that contains all the possible variants that your application will need to transmit messages.
/// This is used to communicate between the different parts of your application.
/// If your application does not need to send messages, you can use an empty enum or `()`.
#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    ToggleAirPlay(bool),
}

/// Implement the `Application` trait for your application.
/// This is where you define the behavior of your application.
///
/// The `Application` trait requires you to define the following types and constants:
/// - `Executor` is the async executor that will be used to run your application's commands.
/// - `Flags` is the data that your application needs to use before it starts.
/// - `Message` is the enum that contains all the possible variants that your application will need to transmit messages.
/// - `APP_ID` is the unique identifier of your application.
impl Application for AirTray {
    type Executor = cosmic::executor::Default;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = "com.github.introini.airtray";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// This is the entry point of your application, it is where you initialize your application.
    ///
    /// Any work that needs to be done before the application starts should be done here.
    ///
    /// - `core` is used to passed on for you by libcosmic to use in the core of your own application.
    /// - `flags` is used to pass in any data that your application needs to use before it starts.
    /// - `Command` type is used to send messages to your application. `Command::none()` can be used to send no messages to your application.
    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let app = AirTray {
            core,
            popup: None,
            airplay_toggle: false,
            uxplay_process: Uxplay::new(),
            ..Default::default()
        };

        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    /// This is the main view of your application, it is the root of your widget tree.
    ///
    /// The `Element` type is used to represent the visual elements of your application,
    /// it has a `Message` associated with it, which dictates what type of message it can send.
    ///
    /// To get a better sense of which widgets are available, check out the `widget` module.
    fn view(&self) -> Element<Self::Message> {
        self.core
            .applet
            .icon_button("com.github.introini.airtray")
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        let content_list = widget::list_column()
            .padding(5)
            .spacing(0)
            .add(settings::item(
                fl!("airplay"),
                widget::toggler(self.airplay_toggle).on_toggle(Message::ToggleAirPlay),
            ));

        self.core.applet.popup_container(content_list).into()
    }

    /// Application messages are handled here. The application state can be modified based on
    /// what message was received. Commands may be returned for asynchronous execution on a
    /// background thread managed by the application's executor.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(372.0)
                        .min_width(300.0)
                        .min_height(200.0)
                        .max_height(1080.0);
                    get_popup(popup_settings)
                }
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::ToggleAirPlay(toggled) => {
                self.airplay_toggle = toggled;
                let _ = self.uxplay_process.manage_uxplay_process();
                if let Err(e) = self.uxplay_process.set_airplay(self.airplay_toggle) {
                    eprintln!("Failed to set airplay: {}", e);
                }
            },
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }

}
