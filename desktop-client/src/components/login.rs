// use iced::{
//     alignment,
//     widget::{button, column, row, text, text_input},
//     Element, Length, Renderer,
// };
// use iced_lazy::Component;
// use lazy_static::lazy_static;
// use regex::Regex;
//
// use crate::Message;
//
// lazy_static! {
//     static ref PHONE_REGEX: Regex = Regex::new(r"^(\d{1-3}$").expect("failed to compile regex");
// }
//
// #[derive(Debug, Default)]
// pub struct State {
//     country: String,
//     phone: String,
//     password: String,
//     device_name: String,
// }
//
// #[derive(Debug, Clone)]
// pub enum Action {
//     CountryChanged(String),
//     PhoneChanged(String),
//     PasswordChanged(String),
//     DeviceNameChanged(String),
//     Login,
// }
//
// #[derive(Debug, Default)]
// pub struct Login;
//
// impl Login {
//     fn is_valid(&self, state: &State) -> bool {
//         !state.phone.is_empty() && !state.device_name.is_empty()
//     }
//
//     fn get_login_button(&self, state: &State) -> Element<'_, Action, Renderer> {
//         let button = button("Login").width(Length::Fill).padding(10);
//
//         let button = if self.is_valid(state) {
//             button.on_press(Action::Login)
//         } else {
//             button
//         };
//         button.into()
//     }
//
//     fn get_text_field(
//         &self,
//         label: &str,
//         value: &str,
//         on_change: impl Fn(String) -> Action + 'static,
//         state: &State,
//     ) -> Element<'_, Action, Renderer> {
//         let input = text_input(label, value).on_input(on_change);
//         if self.is_valid(state) {
//             input.on_submit(Action::Login)
//         } else {
//             input
//         }
//         .into()
//     }
//
//     fn clean_phone(&self, phone: &str) -> String {
//         PHONE_REGEX.replace_all(phone, "").to_string()
//     }
// }
//
// impl Component<Message, Renderer> for Login {
//     type State = State;
//     type Event = Action;
//
//     fn update(&mut self, state: &mut Self::State, event: Self::Event) -> Option<Message> {
//         match event {
//             Action::PhoneChanged(value) => state.phone = self.clean_phone(&value),
//             Action::CountryChanged(_) => todo!(),
//             Action::DeviceNameChanged(value) => state.device_name = value,
//             Action::PasswordChanged(value) => state.password = value,
//             Action::Login => todo!(),
//         }
//         None
//     }
//
//     fn view(&self, state: &Self::State) -> Element<'_, Self::Event, Renderer> {
//         row![column![
//             text("Login")
//                 .width(Length::Fill)
//                 .horizontal_alignment(alignment::Horizontal::Center)
//                 .height(40),
//             row![
//                 self.get_text_field("Country", &state.country, Action::CountryChanged, state),
//                 self.get_text_field("Phone", &state.phone, Action::PhoneChanged, state),
//                 self.get_text_field("Password", &state.password, Action::PasswordChanged, state),
//             ],
//             self.get_text_field(
//                 "Device Name",
//                 &state.device_name,
//                 Action::DeviceNameChanged,
//                 state
//             ),
//             self.get_login_button(state),
//         ]]
//         .into()
//     }
// }
