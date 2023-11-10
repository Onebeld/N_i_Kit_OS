use std::time::Duration;
use dptree::{case, deps};
use is_url::is_url;
use teloxide::{
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
    prelude::*,
    Bot,
    utils::command::BotCommands,
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    types::{InputFile}
};
use crate::website_checker::Website;

mod database;
mod website_checker;

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
type MyDialogue = Dialogue<State, InMemStorage<State>>;

#[derive(Clone)]
pub struct UserWebsites {
    pub user_id: u64,
    pub links: Vec<String>
}

const SECONDS: u64 = 120;

const STICKER_WELCOME: &str = "CAACAgIAAxkBAAEne6RlSyQM7sJfMXWBN3u-dfEgIlxzoAACBQADwDZPE_lqX5qCa011MwQ";
const STICKER_ERROR: &str = "CAACAgIAAxkBAAEne6JlSyP9VdH3N8Mk2imfp7BgFRu9NwACEAADwDZPE-qBiinxHwLoMwQ";

static mut CHECKERS: Vec<UserWebsites> = Vec::new();

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Поддерживаются следующие команды:")]
enum Command {
    #[command(description = "запускает процедуру.")]
    Start,
    #[command(description = "показать меню бота.")]
    Menu,
    #[command(description = "отменяет ввод ссылки.")]
    Cancel,

    #[command(description = "get in")]
    GetInfo,

    #[command(description = "показать комманды.")]
    Help
}

#[derive(Clone, Default)]
enum State {
    #[default]
    Default,
    ReceiveLink,
    ReceiveConfirmRemoveHistories,
    DeletingSomeHistory
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


/// Creates a separate standalone thread in which it checks the availability of sites in the
/// database every hour and if it is unavailable, informs the user
///
/// # Arguments
///
/// * `bot`: Bot instance
unsafe fn launch_checkers(bot: Bot) {
    let histories = database::get_structured_all_histories();

    for history in histories {
        let checker: UserWebsites = UserWebsites {user_id: history.user_id, links: history.links };
        CHECKERS.push(checker);
    }

    let mut interval = tokio::time::interval(Duration::from_secs(SECONDS));

    tokio::spawn(async move {
        loop {
            interval.tick().await;

            let checkers = CHECKERS.clone();

            for checker in checkers {
                let user_id: UserId = UserId(checker.user_id);

                for link in checker.links {
                    let text = format!("Произошла ошибка на стороне сервера по ссылке: {}", link);

                    let _ = bot.send_message(user_id, text).await;
                    let _ = bot.send_sticker(user_id, InputFile::file_id(STICKER_ERROR)).await;

                    let status_code = Website::get_request_code(link.as_str());

                    match status_code {
                        Ok(status_code_unwrapped) => {
                            match status_code_unwrapped {
                                404 => {

                                }
                                200 => { continue }
                                _ => {}
                            }
                        }
                        Err(_) => {
                            let _ = bot.send_message(user_id, format!("Не удалось проверить сайт по ссылке: {}", link)).await;
                        }
                    }
                }
            }
        }
    });
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![State::Default]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Menu].endpoint(show_menu_choice))
            .branch(case![Command::Help].endpoint(help)))
        .branch(case![State::ReceiveLink]
            .branch(case![Command::Cancel].endpoint(cancel_receive_link)));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::ReceiveLink].endpoint(receive_link));

    let callback_query_handler = Update::filter_callback_query()
        .branch(case![State::Default].endpoint(menu_choice_callback_handler))
        .branch(case![State::ReceiveConfirmRemoveHistories].endpoint(menu_confirm_remove_histories_callback_handler));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
}

// Command functions
async fn start(bot: Bot, msg: Message) -> HandlerResult {
    let text = format!("Привет, {}! Я - {}, и я могу проанализировать Ваш сайт, то есть проверить скорость его загрузки и ежечасно приводить отчёт о сбоях в работе указанного Вами сайта.", msg.from().unwrap().first_name, bot.get_me().await?.first_name);
    let keyboard = create_beginning_menu_keyboard().await;

    bot.send_sticker(msg.chat.id, InputFile::file_id(STICKER_WELCOME)).await?;
    bot.send_message(msg.chat.id, text).reply_markup(keyboard).await?;

    Ok(())
}

async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
    Ok(())
}

async fn show_menu_choice(bot: Bot, msg: Message) -> HandlerResult {
    show_actions(bot, msg).await?;

    Ok(())
}

async fn menu_choice_callback_handler(bot: Bot, dialogue: MyDialogue, q: CallbackQuery) -> HandlerResult {
    if let Some(data) = q.data {
        if let Some(message) = q.message {
            match data.as_str() {
                "begin" => show_actions(bot, message).await?,
                "get_history" => get_histories(bot, message, q.from.id).await?,
                "clear_history" => ask_about_clear_histories(bot, dialogue, message).await?,

                "enter_link" => start_enter_link(bot, dialogue, message).await?,

                _ => (),
            }
        }
    }

    Ok(())
}

async fn menu_confirm_remove_histories_callback_handler(bot: Bot, dialogue: MyDialogue, q: CallbackQuery) -> HandlerResult {
    if let Some(data) = q.data {
        if let Some(message) = q.message {
            match data.as_str() {
                "confirm" => clear_histories(bot, dialogue, message, q.from.id).await?,
                "cancel" => {
                    bot.send_message(message.chat.id, "Процесс очистки истории запросов отменен.").await?;
                    dialogue.update(State::Default).await?;
                },

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

async fn ask_about_clear_histories(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let text = "Вы действительно хотите очистить историю запросов?";
    let keyboard = create_confirmation_menu_keyboard().await;

    bot.send_message(msg.chat.id, text).reply_markup(keyboard).await?;

    dialogue.update(State::ReceiveConfirmRemoveHistories).await?;

    Ok(())
}


async fn start_enter_link(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Пожалуйста, введите ссылку.").await?;
    dialogue.update(State::ReceiveLink).await?;

    Ok(())
}

async fn receive_link(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let mut url: String;

    match msg.text() {
        Some(text) => {
            url = text.to_string();
        }
        None => {
            bot.send_message(msg.chat.id, "Пожалуйста, введите ссылку.").await?;
            return Ok(());
        }
    }

    if !Website::has_http_s(url.as_str()) {
        url = format!("https://{}", url);
    }

    if is_url(url.as_str()) {
        if database::is_history_exists(msg.from().unwrap().id.0 as f64, url.as_str()) {
            bot.send_message(msg.chat.id, "Данная ссылка уже была добавлена").await?;

            dialogue.update(State::Default).await?;

            return Ok(());
        }

        database::add_history(msg.from().unwrap().id.0 as f64, url.as_str());

        unsafe {
            add_history_to_checkers(msg.from().unwrap().id.0, url);
        }

        bot.send_message(msg.chat.id, "Спасибо за ссылку!").await?;

        dialogue.update(State::Default).await?;
    }
    else {
        bot.send_message(msg.chat.id, "Данный текст не является ссылкой!").await?;
    }

    Ok(())
}

async fn cancel_receive_link(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Вы отменили ввод ссылки").await?;

    dialogue.update(State::Default).await?;

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

    let confirmation = InlineKeyboardButton::callback("Очистить", "confirm");
    let cancel = InlineKeyboardButton::callback("Отмена", "cancel");

    keyboard.push(vec![confirmation]);
    keyboard.push(vec![cancel]);

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
            CHECKERS.push(UserWebsites {
                user_id,
                links: vec![link]
            });
        }
    }
}

unsafe fn remove_histories_from_checkers(user_id: u64) {
    let index_result = CHECKERS.iter().position(|r| r.user_id == user_id);

    match index_result {
        Some(index) => {
            CHECKERS.remove(index)
        }
        None => {
            return;
        }
    };
}

async fn get_histories(bot: Bot, msg: Message, user_id: UserId) -> HandlerResult {
    let histories = database::get_histories(user_id.0 as f64, None);

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

async fn clear_histories(bot: Bot, dialogue: MyDialogue, msg: Message, user_id: UserId) -> HandlerResult {
    database::clear_histories(user_id.0 as f64);

    unsafe {
        remove_histories_from_checkers(user_id.0);
    }

    bot.send_message(msg.chat.id, "Ваша история запросов успешно очищена!").await?;

    dialogue.update(State::Default).await?;

    Ok(())
}
//