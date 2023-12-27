use std::{
    sync::{Arc, Mutex as StdMutex},
    time::{Duration, SystemTime},
};

use mambembe_keyring::MambembeKeyringError;
use mambembe_lib::{client::AuthyClientApi, models::AuthenticatorToken, AuthyClient};
use relm4::{
    component::{AsyncComponentParts, SimpleAsyncComponent},
    factory::{AsyncFactoryVecDeque, AsyncFactoryVecDequeGuard},
    gtk,
    gtk::{gdk, gio, prelude::*, subclass::prelude::WidgetClassSubclassExt, CssProvider},
    loading_widgets::LoadingWidgets,
    spawn_blocking, tokio,
    tokio::sync::RwLock,
    view, AsyncComponentSender, Component, ComponentController, Controller,
};
use relm4_components::alert::{Alert, AlertMsg, AlertResponse, AlertSettings};
use tokio::{
    sync::{broadcast, broadcast::Sender},
    time::{interval, interval_at, timeout, Instant},
};
use tracing::error;

use crate::{
    components::token_entry::{TokenEntry, TokenInit},
    result::Error,
};

mod components;
mod result;

#[derive(Debug)]
#[tracker::track]
struct MambembeDesktop {
    #[do_not_track]
    authy_client: Option<Arc<RwLock<AuthyClient>>>,
    // state: ApplicationState,
    #[do_not_track]
    tokens: AsyncFactoryVecDeque<TokenEntry>,
    phone: String,
    // phone_state: text_input::State,
    // device_name_state: text_input::State,
    // device_name: String,
    #[do_not_track]
    keyring_alert: Controller<Alert>,
    #[do_not_track]
    error_alert: Controller<Alert>,

    #[do_not_track]
    receiver: broadcast::Receiver<()>,
    #[do_not_track]
    sender: broadcast::Sender<()>,

    #[do_not_track]
    current_filter: Arc<StdMutex<String>>,
    #[do_not_track]
    tokens_widget: gtk::ListBox,

    timer_value: u8,
}

#[derive(Debug)]
pub enum AppInputMessage {
    LoginNeeded,
    Error(Error),
    Ignore,
    Close,
    LoadClientFromKeyRing,
    TimeSync,
    RefreshTokens,
    DecrementTimer,
    SetTimer(u8),
    RefreshCurrentOtp,
    Filter(String),
}

#[relm4::component(async)]
impl SimpleAsyncComponent for MambembeDesktop {
    type Input = AppInputMessage;
    type Output = ();
    type Init = ();

    view! {
        #[root]
        #[name(window)]
        gtk::ApplicationWindow {
            set_icon_name: Some("applications-utilities-symbolic"),
            set_widget_name: "main-window",
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::Box {
                    set_spacing: 20,
                    gtk::SearchEntry {
                        set_hexpand: true,
                        set_placeholder_text: Some("Filter tokens"),
                        connect_search_changed[cloned_sender] => move |entry| {
                            let text = entry.text().to_string();
                            cloned_sender.input(AppInputMessage::Filter(text));
                        },
                    },
                    gtk::Button {
                        set_margin_end: 2,
                        connect_clicked[cloned_sender] => move |_| {
                                cloned_sender.input(AppInputMessage::RefreshCurrentOtp);
                        },
                        gtk::Box {
                            set_spacing: 10,
                            set_width_request: 100,
                            gtk::Image {
                                set_margin_start: 28,
                                set_from_icon_name: Some("view-refresh-symbolic"),
                            },

                            gtk::Label {
                                #[track = "model.changed(Self::timer_value())"]
                                set_label: &model.get_timer_value().to_string(),
                            },
                        }

                    },
                },
                gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_widget_name: "tokens-scrolled-window",

                    #[local_ref]
                    tokens_list -> gtk::ListBox {
                        set_filter_func[current_filter] => move |item| {
                            let text = current_filter.lock().unwrap();
                            item.widget_name().to_lowercase().contains(&*text)
                        },
                    }
                },

            }
        }
    }

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let authy_client = Self::get_client_from_keyring(sender.clone()).await;
        // Self::sync_time(sender.clone(), authy_client.as_ref()).await;

        let keyring_alert_sender = sender.clone();
        let keyring_alert = Alert::builder()
            .transient_for(&root)
            .launch(AlertSettings {
                text: "Refusing to keep working without a valid keyring".to_string(),
                secondary_text: None,
                is_modal: true,
                destructive_accept: false,
                confirm_label: "Try again?".to_string(),
                cancel_label: "Cancel".to_string(),
                option_label: None,
            })
            .forward(
                keyring_alert_sender.input_sender(),
                |response| match response {
                    AlertResponse::Confirm => AppInputMessage::LoadClientFromKeyRing,
                    AlertResponse::Cancel => AppInputMessage::Close,
                    _ => AppInputMessage::Ignore,
                },
            );
        let error_alert = Alert::builder()
            .transient_for(&root)
            .launch(AlertSettings {
                text: "An unknown error happened".to_string(),
                secondary_text: None,
                is_modal: true,
                destructive_accept: false,
                confirm_label: "Try again?".to_string(),
                cancel_label: "Cancel".to_string(),
                option_label: None,
            })
            .forward(sender.input_sender(), |response| match response {
                AlertResponse::Confirm => AppInputMessage::LoadClientFromKeyRing,
                AlertResponse::Cancel => AppInputMessage::Close,
                _ => AppInputMessage::Ignore,
            });

        let mut tokens =
            AsyncFactoryVecDeque::<TokenEntry>::new(gtk::ListBox::default(), sender.input_sender());

        let (senderz, receiver) = broadcast::channel(1);

        let app_sender = sender.clone();
        tokio::spawn(async move {
            // Get interval for the next minute
            let t = SystemTime::now();
            let unix_timestamp = t
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis();
            let current_seconds = unix_timestamp % 60_000;
            let next_minute = 60_000 - current_seconds;
            app_sender.input(AppInputMessage::SetTimer((next_minute / 1000) as u8));
            let first_tick = Duration::from_millis(next_minute as u64);
            let mut update_at = if first_tick.as_millis() > 0 {
                interval_at(Instant::now() + first_tick, Duration::from_secs(60))
            } else {
                interval(Duration::from_secs(60))
            };
            loop {
                match timeout(Duration::from_secs(1), update_at.tick()).await {
                    Ok(_) => {
                        app_sender.input(AppInputMessage::DecrementTimer);
                        app_sender.input(AppInputMessage::RefreshCurrentOtp);
                    }
                    Err(_) => app_sender.input(AppInputMessage::DecrementTimer),
                }
            }
        });

        Self::refresh_tokens(
            sender.clone(),
            &mut tokens.guard(),
            authy_client.as_ref(),
            senderz.clone(),
        )
        .await;

        let current_filter = Arc::new(StdMutex::new("".to_string()));
        let tokens_list = tokens.widget().clone();

        let model = Self {
            authy_client,
            tokens,
            phone: "".to_string(),
            keyring_alert,
            error_alert,
            sender: senderz,
            receiver,
            timer_value: 60,
            tokens_widget: tokens_list.clone(),
            tracker: 0,
            current_filter: current_filter.clone(),
        };

        let cloned_sender = sender.clone();

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    fn init_loading_widgets(root: &mut Self::Root) -> Option<LoadingWidgets> {
        let provider = CssProvider::new();
        provider.load_from_resource("/mambembe/main.css");

        let display = gdk::Display::default().expect("Could not connect to a display.");
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        view! {
            #[local_ref]
            root {
                set_title: Some("Mambembe"),
                set_default_size: (600, 800),

                #[name(temp)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_spacing: 20,
                    gtk::Label {
                        set_label: "Loading data from keyring...",
                    },
                    gtk::Spinner {
                        start: (),
                        set_halign: gtk::Align::Center,
                    }
                }
            }
        }
        Some(LoadingWidgets::new(root, temp))
    }

    async fn update(&mut self, message: Self::Input, sender: AsyncComponentSender<Self>) {
        self.reset();
        match message {
            AppInputMessage::LoginNeeded => {
                todo!("Finish login");
            }
            AppInputMessage::Error(err) => {
                error!(?err);

                match err {
                    Error::KeyRing(_) => {
                        self.keyring_alert.sender().emit(AlertMsg::Show);
                    }
                    _ => {
                        self.error_alert.sender().emit(AlertMsg::Show);
                    }
                }
            }
            AppInputMessage::Ignore => {}
            AppInputMessage::Close => relm4::main_application().quit(),
            AppInputMessage::LoadClientFromKeyRing => {
                let authy_client = Self::get_client_from_keyring(sender.clone()).await;
                if authy_client.is_some() {
                    sender.input(AppInputMessage::RefreshTokens);
                }
                self.authy_client = authy_client;
            }
            AppInputMessage::RefreshTokens => {
                let mut guard = self.tokens.guard();
                let client = self.authy_client.as_ref();
                let refresh_token_sender = self.sender.clone();

                Self::refresh_tokens(sender.clone(), &mut guard, client, refresh_token_sender)
                    .await;
            }
            AppInputMessage::DecrementTimer => {
                if let Some(new_value) = self.timer_value.checked_sub(1) {
                    self.set_timer_value(new_value);
                } else {
                    self.set_timer_value(60);
                }
            }
            AppInputMessage::SetTimer(value) => {
                self.set_timer_value(value);
            }
            AppInputMessage::RefreshCurrentOtp => {
                self.sender
                    .send(())
                    .expect("should not fail as component carry a receiver");
            }
            AppInputMessage::TimeSync => {
                Self::sync_time(sender, self.authy_client.as_ref()).await;
            }
            AppInputMessage::Filter(filter) => {
                let current_filter = self.current_filter.clone();
                spawn_blocking(move || {
                    *current_filter.lock().expect("poisoned lock") = filter.to_lowercase();
                })
                .await
                .expect("failed to set filter");
                self.tokens_widget.invalidate_filter();
            }
        }
    }
}

impl MambembeDesktop {
    async fn get_client_from_keyring(
        sender: AsyncComponentSender<MambembeDesktop>,
    ) -> Option<Arc<RwLock<AuthyClient>>> {
        dbg!(
            spawn_blocking(move || match mambembe_keyring::get::<AuthyClient>() {
                Ok(client) => dbg!(Some(Arc::new(RwLock::new(client)))),
                Err(MambembeKeyringError::NoPasswordFound) => {
                    sender.input(AppInputMessage::LoginNeeded);
                    None
                }
                Err(e) => {
                    sender.input(AppInputMessage::Error(e.into()));
                    None
                }
            })
            .await
            .expect("Unexpected error getting client")
        )
    }

    async fn create_token_entries_from_refreshed_tokens<'a>(
        sender: AsyncComponentSender<MambembeDesktop>,
        guard: &mut AsyncFactoryVecDequeGuard<'a, TokenEntry>,
        client: &Arc<RwLock<AuthyClient>>,
        refresh_token: broadcast::Sender<()>,
    ) {
        let tokens = spawn_blocking(move || mambembe_keyring::get::<Vec<AuthenticatorToken>>())
            .await
            .expect("Unexpected error getting tokens from keyring");

        let tokens = match tokens {
            Ok(tokens) => tokens,
            Err(MambembeKeyringError::NoPasswordFound) => {
                println!("Tokens not found on keyring, refreshing...");
                match client.read().await.list_authenticator_tokens().await {
                    Ok(tokens) => tokens,
                    Err(err) => {
                        sender.input(AppInputMessage::Error(err.into()));
                        return;
                    }
                }
            }
            Err(err) => {
                sender.input(AppInputMessage::Error(err.into()));
                return;
            }
        };

        let tokens = spawn_blocking(move || {
            mambembe_keyring::set(&tokens).expect("Unexpected error saving tokens to keyring");
            tokens
        })
        .await
        .expect("Unexpected error saving tokens to keyring");

        for mut token in tokens {
            guard.push_back(TokenInit {
                token,
                client: client.clone(),
                refresh_token: refresh_token.subscribe(),
            });
        }
    }

    async fn sync_time(
        sender: AsyncComponentSender<MambembeDesktop>,
        authy_client: Option<&Arc<RwLock<AuthyClient>>>,
    ) {
        if let Some(client) = authy_client {
            if let Err(e) = client.write().await.sync_time_with_server().await {
                panic!("{e:?}");
                sender.input(AppInputMessage::Error(e.into()));
            }
        }
    }

    async fn refresh_tokens<'a>(
        sender: AsyncComponentSender<MambembeDesktop>,
        mut guard: &mut AsyncFactoryVecDequeGuard<'a, TokenEntry>,
        client: Option<&Arc<RwLock<AuthyClient>>>,
        refresh_token_sender: Sender<()>,
    ) {
        if let Some(client) = client {
            Self::create_token_entries_from_refreshed_tokens(
                sender,
                &mut guard,
                client,
                refresh_token_sender,
            )
            .await
        }
    }
}

fn main() {
    gio::resources_register_include!("main.gresource").expect("failed to register resources");
    let gtk_app = gtk::Application::builder()
        .application_id("br.com.jayson.mambembe-desktop")
        .build();

    #[cfg(target_os = "macos")]
    {
        // gtk::Text::add_shortcut(gtk::Shortcut::new(
        //     ShortcutTrigger::parse_string("<Meta>a"),
        //     ShortcutAction::parse_string("select-all"),
        // ));
        // dbg!("aaa");
        gtk_app.set_accels_for_action("selection.select-all", &["<meta>a"]);
        gtk_app.set_accels_for_action("window.close", &["<meta>q"]);
    }
    #[cfg(not(target_os = "macos"))]
    {
        gtk_app.set_accels_for_action("selection.select-all", &["<primary>a"]);
        gtk_app.set_accels_for_action("window.close", &["<primary>q"]);
    }

    let app = relm4::RelmApp::from_app(gtk_app);
    app.run_async::<MambembeDesktop>(())
}
