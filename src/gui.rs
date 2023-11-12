use iced::{executor, Application, Command, Element, Settings, Theme};

pub fn start(db: sqlx::Pool::<sqlx::Sqlite>) -> Result<(), GuiError> {
	Hello::run(Settings::default())?;
	Ok(())
}

struct Hello;

impl Application for Hello {
    type Executor = executor::Default;

    type Message = ();

    type Theme = Theme;

    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (Self, Command::none())
    }

    fn title(&self) -> String {
        String::from("rustodo")
    }

    fn update(&mut self, _message: Self::Message) -> Command<Self::Message> {
        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        "Hello, World!".into()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum GuiError {
	#[error("Iced error: {0}")]
	IcedError(#[from] iced::Error),
}
