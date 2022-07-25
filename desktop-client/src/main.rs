use std::sync::Arc;

use iced::{
    futures::lock::Mutex,
    text_input::{self, State},
    Application, Clipboard, Column, Command, Container, HorizontalAlignment, Length, Settings,
    Text, TextInput,
};
use mambembe_lib::{
    client::AuthyClientApi, models::AuthenticatorToken, AuthyClient, MambembeError,
    Result as LibResult,
};

struct MambembeDesktop {
    authy_client: Option<Arc<Mutex<AuthyClient>>>,
    state: ApplicationState,
    tokens: Vec<AuthenticatorToken>,
    phone: String,
    phone_state: text_input::State,
    device_name_state: text_input::State,
    device_name: String,
}

enum ApplicationState {
    Loading,
    Loaded,
    RegisteringDevice,
}

#[derive(Debug)]
enum Message {
    Initialized(LibResult<AuthyClient>),
    FailedToInitialize(MambembeError),
    DeviceChecked(LibResult<()>),
    TokensFetched(LibResult<Vec<AuthenticatorToken>>),
    // InputChanged(String),
}

impl Application for MambembeDesktop {
    type Executor = iced::executor::Default;
    type Message = Message;

    type Flags = ();

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                authy_client: None,
                state: ApplicationState::Loading,
                tokens: vec![],
                phone: String::new(),
                phone_state: text_input::State::new(),
                device_name: String::new(),
                device_name_state: text_input::State::new(),
            },
            Command::perform(AuthyClient::from_file(), Message::Initialized),
        )
    }

    fn title(&self) -> String {
        "MambembeDesktop".to_string()
    }

    fn update(&mut self, message: Self::Message, _: &mut Clipboard) -> Command<Self::Message> {
        match message {
            Message::Initialized(r) => match r {
                Ok(client) => {
                    self.authy_client = Some(Arc::new(Mutex::new(client)));
                    let client = { self.authy_client.as_ref().unwrap().clone() };
                    let future = async move {
                        let client = client.lock().await;
                        client.check_current_device().await
                    };
                    return Command::perform(future, Message::DeviceChecked);
                }
                Err(MambembeError::ConfigFileNotFound(_)) => {
                    self.state = ApplicationState::RegisteringDevice;
                }
                Err(err) => panic!("{}", err),
            },
            Message::FailedtoInitialize(err) => {
                panic!(err);
            }
            Message::DeviceChecked(result) => match result {
                Ok(_) => {
                    let client = { self.authy_client.as_ref().unwrap().clone() };

                    return Command::perform(
                        async move {
                            let client = client.lock().await;
                            client.list_authenticator_tokens().await
                        },
                        Message::TokensFetched,
                    );
                }
                Err(err) => panic!("{}", err),
            },
            Message::TokensFetched(result) => match result {
                Ok(tokens) => {
                    self.tokens = tokens;
                    self.state = ApplicationState::Loaded;
                }
                err => {
                    err.unwrap();
                }
            },
        };
        Command::none()
    }

    fn view(&mut self) -> iced::Element<'_, Self::Message> {
        match self.state {
            ApplicationState::Loading => Container::new(
                Text::new("Loading...")
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .size(50),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_y()
            .into(),
            ApplicationState::Loaded => {
                let title = Text::new("mambembe")
                    .width(Length::Fill)
                    .size(100)
                    .color([0.5, 0.5, 0.5])
                    .horizontal_alignment(HorizontalAlignment::Center);

                Container::new(Column::new().push(title).push(self.build_token_list())).into()
            }
            ApplicationState::RegisteringDevice => {
                // Container::new(
                //     Column::new()
                //         .push(Text::new("Registering"))
                //         .push(TextInput::new(
                //             &mut self.phone_state,
                //             "phone",
                //             &self.phone,
                //             noop,
                //         ))
                //         .push(TextInput::new(
                //             &mut self.device_name_state,
                //             "device name",
                //             &self.device_name,
                //             noop,
                //         )),
                // )
                // .into()
                todo!()
            }
        }
    }
}

impl MambembeDesktop {
    fn build_token_list(&self) -> Column<Message> {
        let mut column = Column::new();

        for token in self.tokens.iter() {
            column = column.push(Text::new(&token.name));
        }

        column
    }
}

fn noop<T>(input: T) {}

fn main() -> iced::Result {
    MambembeDesktop::run(Settings::default())
}
