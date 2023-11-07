use std::time::Duration;
use dptree::{case, deps};
use teloxide::{
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
    prelude::*,
    Bot,
    utils::command::BotCommands,
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler}
};
use crate::checker::Checker;
use crate::database::Database;
use crate::website_checker::Website;

mod database;
mod website_checker;
mod checker;

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
type MyDialogue = Dialogue<State, InMemStorage<State>>;

static mut CHECKERS: Vec<Checker> = Vec::new();

// 3600
static SECONDS: u64 = 5;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "start the procedure.")]
    Start,
    #[command(description = "start the procedure without welcome message.")]
    QuickStart,
    #[command(description = "show bot's menu.")]
    Menu,

    #[command(description = "show commands.")]
    Help
}

#[derive(Clone, Default)]
enum State {
    #[default]
    Default,

    ReceiveLink,
}

#[tokio::main]
async fn main() -> HandlerResult {
    let bot = Bot::from_env();

    unsafe {
        launch_checkers(bot.clone());
    }

    Dispatcher::builder(bot, schema())
        .dependencies(deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

unsafe fn launch_checkers(bot: Bot) {
    let histories = Database::get_structured_all_histories();

    for history in histories {
        let checker: Checker = Checker::new(history.user_id, history.links);
        CHECKERS.push(checker);
    }

    let mut interval = tokio::time::interval(Duration::from_secs(SECONDS));

    tokio::spawn(async move {
        loop {
            interval.tick().await;

            let checkers = CHECKERS.clone();

            for mut checker in checkers {
                if !checker.is_activated {
                    continue;
                }

                checker.check_websites();
            }
        }
    });
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![State::Default]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Menu].endpoint(show_menu_choice)));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::ReceiveLink].endpoint(recive_link));

    let callback_query_handler = Update::filter_callback_query()
        .branch(case![State::Default].endpoint(menu_choice_callback_handler));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
}

// Command functions
async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let text = format!("Привет, {}! Я - {}, и я могу проанализировать Ваш сайт, то есть проверить скорость его загрузки и ежечасно приводить отчёт о сбоях в работе указанного Вами сайта.", msg.from().unwrap().first_name, bot.get_me().await?.first_name);
    let keyboard = create_beginning_menu_keyboard().await;

    bot.send_message(msg.chat.id, text).reply_markup(keyboard).await?;

    Ok(())
}

async fn show_menu_choice(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    show_actions(bot, msg).await?;

    Ok(())
}

async fn menu_choice_callback_handler(bot: Bot, dialogue: MyDialogue, q: CallbackQuery) -> HandlerResult {
    if let Some(data) = q.data {
        if let Some(message) = q.message {
            match data.as_str() {
                "begin" => show_actions(bot, message).await?,
                "get_history" => get_histories(bot, message, q.from.id).await?,
                "clear_history" => clear_histories(bot, message, q.from.id).await?,
                "confirm_clear_history" => ask_about_clear_histories(bot, message).await?,

                "enter_link" => start_enter_link(bot, dialogue, message).await?,

                _ => (),
            }
        }
    }

    Ok(())
}

async fn show_actions(bot: Bot, msg: Message) -> HandlerResult {
    let text = "Что вы хотите сделать?";
    let keyboard = create_main_menu_keyboard().await;

    bot.send_message(msg.chat.id, text).reply_markup(keyboard).await?;

    Ok(())
}

async fn ask_about_clear_histories(bot: Bot, msg: Message) -> HandlerResult {
    let text = "Вы действительно хотите очистить истоирю запросов? Нажмите на кнопку ниже, если вы подтверждаете своё действие.";
    let keyboard = create_confirmation_menu_keyboard().await;

    bot.send_message(msg.chat.id, text).reply_markup(keyboard).await?;

    Ok(())
}


async fn start_enter_link(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Пожалуйста, введите ссылку.").await?;
    dialogue.update(State::ReceiveLink).await?;

    Ok(())
}

async fn recive_link(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            let res = Website::try_parse(text.to_string());

            match res {
                Ok(link) => {
                    bot.send_message(msg.chat.id, "Спасибо за ссылку!").await?;

                    Database::add_history(msg.from().unwrap().id.0 as f64, link.to_string().as_str());

                    unsafe {
                        add_history_to_checkers(msg.from().unwrap().id.0, link.to_string());
                    }

                    dialogue.update(State::Default).await?;
                }
                Err(_) => {
                    bot.send_message(msg.chat.id, "Данный текст не является ссылкой!").await?;
                }
            }
        }
        None => {
            bot.send_message(msg.chat.id, "Пожалуйста, введите ссылку.").await?;
        }
    }

    Ok(())
}

// Creating bot menus
async fn create_beginning_menu_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    let begin = InlineKeyboardButton::callback("Приступим!", "begin");

    keyboard.push(vec![begin]);

    InlineKeyboardMarkup::new(keyboard)
}

async fn create_main_menu_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    let enter_link = InlineKeyboardButton::callback("Ввести ссылку на сайт", "enter_link");
    let histories = InlineKeyboardButton::callback("Получить историю запросов", "get_history");
    let clear_history = InlineKeyboardButton::callback("Очистить историю запросов", "clear_history");

    keyboard.push(vec![enter_link]);
    keyboard.push(vec![histories]);
    keyboard.push(vec![clear_history]);

    InlineKeyboardMarkup::new(keyboard)
}

async fn create_confirmation_menu_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    let confirmation = InlineKeyboardButton::callback("Очистить", "confirm_clear_history");

    keyboard.push(vec![confirmation]);

    InlineKeyboardMarkup::new(keyboard)
}
//

// General features
unsafe fn add_history_to_checkers(user_id: u64, link: String) {
    let index_result = CHECKERS.iter().position(|r| r.user_id == user_id);

    match index_result {
        Some(index) => {
            CHECKERS[index].links.push(link);
        }
        None => {
            CHECKERS.push(Checker {
                user_id,
                is_activated: true,
                links: vec![link]
            });
        }
    }
}

async fn get_histories(bot: Bot, msg: Message, user_id: UserId) -> HandlerResult {
    let histories = Database::get_histories(user_id.0 as f64, None);

    if histories.iter().count() == 0 {
        bot.send_message(msg.chat.id, "У вас нет истории запросов").await?;
    }
    else {
        let mut str = format!("Вот ваша история запросов:\n");

        for i in 0..histories.iter().count() {
            let link = format!("\n[{}] {}", i + 1, histories[i].link);
            str.push_str(link.as_str());
        }

        bot.send_message(msg.chat.id, str).await?;
    }

    Ok(())
}

async fn clear_histories(bot: Bot, msg: Message, user_id: UserId) -> HandlerResult {
    Database::clear_histories(user_id.0 as f64);

    bot.send_message(msg.chat.id, "Ваша история запросов успешно очищена!").await?;

    Ok(())
}
//