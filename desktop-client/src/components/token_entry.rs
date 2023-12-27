use std::sync::Arc;

use mambembe_lib::{client::AuthyClientApi, models::AuthenticatorToken, AuthyClient};
use relm4::{
    factory::AsyncFactoryComponent,
    gtk::prelude::*,
    loading_widgets::LoadingWidgets,
    prelude::*,
    tokio,
    tokio::{
        select,
        sync::{broadcast, RwLock},
    },
    view, AsyncFactorySender,
};
use tokio_util::sync::CancellationToken;

use crate::AppInputMessage;

#[derive(Debug)]
#[tracker::track]
pub struct TokenEntry {
    token_value: String,
    #[do_not_track]
    token: AuthenticatorToken,
    #[do_not_track]
    client: Arc<RwLock<AuthyClient>>,
    #[do_not_track]
    cancellation_token: CancellationToken,
    #[do_not_track]
    current_filter: Arc<std::sync::Mutex<String>>,
}

pub struct TokenInit {
    pub token: AuthenticatorToken,
    pub client: Arc<RwLock<AuthyClient>>,
    pub refresh_token: broadcast::Receiver<()>,
}

#[derive(Debug)]
pub enum TokenInput {
    RefreshToken,
}

#[relm4::factory(pub, async)]
impl AsyncFactoryComponent for TokenEntry {
    type Init = TokenInit;
    type Input = TokenInput;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;
    type ParentInput = AppInputMessage;

    view! {
        gtk::ListBoxRow {
            set_selectable: false,
            set_widget_name: &self.token.name,
            add_css_class: "token-row",

            gtk::Box {
                set_spacing: 20,
                gtk::Label {
                    set_text: &self.token.name,
                    set_hexpand: true,
                    set_halign: gtk::Align::Start,
                    add_css_class: "token-name",
                },
                #[name(token_label)]
                gtk::Label {
                    #[track = "self.changed(Self::token_value())"]
                    set_text: &self.token_value,
                    add_css_class: "token-value",
                },
                #[name="copy_button"]
                gtk::Button {
                    connect_clicked[token_label] => move |button| {
                        button.clipboard().set_text(&token_label.text());
                    },
                    gtk::Box {
                        set_spacing: 10,
                        set_width_request: 100,
                        gtk::Image {
                            set_margin_start: 20,
                            set_icon_name: Some("edit-copy-symbolic"),
                        },
                        gtk::Label {
                            set_text: "Copy",
                        },
                    },
                },
            }
        }
    }

    async fn init_model(
        value: Self::Init,
        _index: &DynamicIndex,
        sender: AsyncFactorySender<Self>,
    ) -> Self {
        let mut token = value.token;
        value
            .client
            .read()
            .await
            .initialize_authenticator_token(&mut token)
            .expect("failed initializing token");

        let token_value = value
            .client
            .read()
            .await
            .get_otp_token(&token)
            .await
            .expect("failed to get token");

        let cancellation_token = CancellationToken::new();
        let cloned = cancellation_token.clone();
        let mut refresh = value.refresh_token;

        tokio::spawn(async move {
            loop {
                select! {
                    _ = refresh.recv() => {
                        sender.input(TokenInput::RefreshToken);
                    }
                    _ = cloned.cancelled() => {
                        break;
                    }
                }
            }
        });

        Self {
            token_value,
            token,
            client: value.client,
            cancellation_token,
            current_filter: Arc::new(std::sync::Mutex::new(String::new())),
            tracker: 0,
        }
    }

    async fn update(&mut self, msg: Self::Input, _sender: AsyncFactorySender<Self>) {
        self.reset();
        match msg {
            TokenInput::RefreshToken => {
                let token_value = self
                    .client
                    .read()
                    .await
                    .get_otp_token(&self.token)
                    .await
                    .expect("failed to get token");

                self.set_token_value(token_value);
            }
        }
    }

    fn init_loading_widgets(root: &mut Self::Root) -> Option<LoadingWidgets> {
        view! {
            #[local_ref]
            root {
                #[name(temp)]
                gtk::Spinner {
                    start: (),
                    set_halign: gtk::Align::Center,
                }
            }
        }
        Some(LoadingWidgets::new(root, temp))
    }
}

impl Drop for TokenEntry {
    fn drop(&mut self) {
        self.cancellation_token.cancel();
    }
}
