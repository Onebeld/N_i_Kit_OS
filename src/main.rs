use std::time::Duration;
use dptree::{case, deps};
use is_url::is_url;
use log::LevelFilter;
use teloxide::{
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
    prelude::*,
    Bot,
    utils::command::BotCommands,
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    types::{InputFile}
};
use crate::database::History;

extern crate pretty_env_logger;
#[macro_use] extern crate log;

mod database;
mod website;

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
type MyDialogue = Dialogue<State, InMemStorage<State>>;

#[derive(Clone)]
pub struct UserWebsites {
    pub user_id: u64,
    pub links: Vec<String>
}

const SECONDS: u64 = 3600;

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
    #[command(description = "отменяет ввод данных в бот.")]
    Cancel,

    #[command(description = "показать команды.")]
    Help
}

#[derive(Clone, Default)]
enum State {
    #[default]
    Default,

    ReceiveLink,
    ReceiveLinkForChecking,
    ReceiveConfirmRemoveHistories,
    DeletingSomeHistory
}

#[tokio::main]
async fn main() -> HandlerResult {
    pretty_env_logger::formatted_timed_builder().filter_level(LevelFilter::Info).init();

    let bot = Bot::from_env();

    info!("The bot is up and running and ready to go!");

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
        let checker: UserWebsites = UserWebsites { user_id: history.user_id, links: history.links };
        CHECKERS.push(checker);
    }

    let mut interval = tokio::time::interval(Duration::from_secs(SECONDS));

    info!("A thread has been launched to test sites.");

    tokio::spawn(async move {
        loop {
            interval.tick().await;

            info!("Runs a site checker");

            let checkers = CHECKERS.clone();

            for checker in checkers {
                let user_id: UserId = UserId(checker.user_id);

                for link in checker.links {
                    let status_code = website::get_request_code(link.as_str()).await;

                    match status_code {
                        Ok(status_code_unwrapped) => {
                            handle_status_code(&bot, checker.user_id, link, status_code_unwrapped).await;
                        }
                        Err(err) => {
                            error!("Failed to verify the site for the user: {}. Description: {}", checker.user_id, err.to_string());
                            let _ = bot.send_message(user_id, format!("Не удалось проверить сайт по ссылке: {}", link)).await;
                        }
                    }
                }
            }
        }
    });
}

async fn handle_status_code(bot: &Bot, user_id: u64, link: String, status_code: u16) {
    let text = format!("Произошла ошибка при проверки ссылки: {}", link);

    match status_code {
        403 => {
            let _ = bot.send_sticker(UserId(user_id), InputFile::file_id(STICKER_ERROR)).await;
            let _ = bot.send_message(UserId(user_id), format!("{text}\n\nКод ошибки: {status_code}\nБот не может получить доступ")).await;
        }
        404 => {
            let _ = bot.send_sticker(UserId(user_id), InputFile::file_id(STICKER_ERROR)).await;
            let _ = bot.send_message(UserId(user_id), format!("{text}\n\nКод ошибки: {status_code}\nЭтого сайта не существует")).await;
        }
        500 => {
            let _ = bot.send_sticker(UserId(user_id), InputFile::file_id(STICKER_ERROR)).await;
            let _ = bot.send_message(UserId(user_id), format!("{text}\n\nКод ошибки: {status_code}\nВнутренняя ошибка сервера")).await;
        }
        503 => {
            let _ = bot.send_sticker(UserId(user_id), InputFile::file_id(STICKER_ERROR)).await;
            let _ = bot.send_message(UserId(user_id), format!("{text}\n\nКод ошибки: {status_code}\nСервис недоступен")).await;
        }

        200..=299 | 300..=399 => { return; }
        _ => {
            let _ = bot.send_sticker(UserId(user_id), InputFile::file_id(STICKER_ERROR)).await;
            let _ = bot.send_message(UserId(user_id), format!("{text}\n\nКод ошибки: {status_code}")).await;
        }
    }
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![State::Default]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Menu].endpoint(show_menu_choice))
            .branch(case![Command::Help].endpoint(help)))
        .branch(case![State::ReceiveLink]
            .branch(case![Command::Cancel].endpoint(cancel_receive_link)))
        .branch(case![State::DeletingSomeHistory]
            .branch(case![Command::Cancel].endpoint(cancel_deleting_some_histories)))
        .branch(case![State::ReceiveLinkForChecking]
            .branch(case![Command::Cancel].endpoint(cancel_receive_link)));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::ReceiveLink].endpoint(receive_link))
        .branch(case![State::DeletingSomeHistory].endpoint(delete_some_histories))
        .branch(case![State::ReceiveLinkForChecking].endpoint(check_site));

    let callback_query_handler = Update::filter_callback_query()
        .branch(case![State::Default].endpoint(menu_choice_callback_handler))
        .branch(case![State::ReceiveConfirmRemoveHistories].endpoint(menu_confirm_remove_histories_callback_handler));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
}

/// Displays a welcome message to the user
///
/// # Arguments
///
/// * `bot`: Bot instance
/// * `msg`: Message sent by the user
///
/// returns: Result<(), Box<dyn Error+Send+Sync, Global>>
async fn start(bot: Bot, msg: Message) -> HandlerResult {
    info!("A new user has joined the bot: {}", msg.from().unwrap().id);

    let text = format!("🚀 Привет, {}! Я - {}, и я могу проанализировать Ваш сайт, то есть проверить скорость его загрузки и ежечасно приводить отчёт о сбоях в работе указанного Вами сайта.", msg.from().unwrap().first_name, bot.get_me().await?.first_name);
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
                "check_link" => start_check_link(bot, dialogue, message).await?,
                "get_history" => get_histories(bot, message, q.from.id).await?,
                "clear_history" => ask_about_clear_histories(bot, dialogue, message).await?,
                "delete_some_histories" => start_deleting_some_histories(bot, dialogue, message, q.from.id).await?,

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
    let text = format!("{} к вашим услугам!\nЧто вы хотите сделать?", bot.get_me().await?.first_name);
    let keyboard = create_main_menu_keyboard().await;

    bot.send_message(msg.chat.id, text).reply_markup(keyboard).await?;

    Ok(())
}

async fn ask_about_clear_histories(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let text = "❓ Вы действительно хотите очистить историю запросов? ❓";
    let keyboard = create_confirmation_menu_keyboard().await;

    bot.send_message(msg.chat.id, text).reply_markup(keyboard).await?;

    dialogue.update(State::ReceiveConfirmRemoveHistories).await?;

    Ok(())
}


async fn start_enter_link(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Пожалуйста, введите ссылку. Для отмены ввода ссылки введите команду /cancel").await?;
    dialogue.update(State::ReceiveLink).await?;

    Ok(())
}

async fn start_check_link(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Пожалуйста, введите ссылку. Для отмены ввода ссылки введите команду /cancel").await?;
    dialogue.update(State::ReceiveLinkForChecking).await?;

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

    if !website::has_http_s(url.as_str()) {
        url = format!("https://{}", url);
    }

    if is_url(url.as_str()) {
        if database::is_history_exists(msg.from().unwrap().id.0, url.as_str()) {
            bot.send_message(msg.chat.id, "Данная ссылка уже была добавлена").await?;

            dialogue.update(State::Default).await?;

            return Ok(());
        }

        database::add_history(msg.from().unwrap().id.0, url.as_str());

        unsafe {
            add_history_to_checkers(msg.from().unwrap().id.0, url);
        }

        info!("Added a new link to the database from the user: {}", msg.from().unwrap().id);

        bot.send_message(msg.chat.id, "Спасибо за ссылку! Теперь я буду проверять эту ссылку каждый час").await?;

        dialogue.update(State::Default).await?;
    }
    else {
        bot.send_message(msg.chat.id, "Данный текст не является ссылкой!").await?;
    }

    Ok(())
}

async fn check_site(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
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

    if !website::has_http_s(url.as_str()) {
        url = format!("https://{}", url);
    }

    if is_url(url.as_str()) {
        info!("Site information for the user is requested: {}", msg.from().unwrap().id.0);

        let certificate_result = website::get_ssl_certificate(url.as_str());

        match certificate_result {
            Ok(cert) => {
                let text = format!("Информация о введеном вами сайте:\n\nОбщее название: {}\nОрганизация: {}\n\
                Страна: {}\nИздатель: {}", cert.intermediate.common_name,
                                   cert.intermediate.organization,
                                   cert.intermediate.country,
                                   cert.intermediate.issuer);

                bot.send_message(msg.chat.id, text).await?;

            }
            Err(_) => {
                bot.send_message(msg.chat.id, "Не удалось проверить введеный вами сайт.").await?;
            }
        }

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

async fn cancel_deleting_some_histories(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Вы отменили удаление истории").await?;

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

    let check_link = InlineKeyboardButton::callback("🔭 Проанализировать сайт 🔭", "check_link");
    let enter_link = InlineKeyboardButton::callback("✏️ Ввести ссылку на сайт ✏️", "enter_link");
    let histories = InlineKeyboardButton::callback("📒 Получить историю запросов 📒", "get_history");
    let delete_some_histories = InlineKeyboardButton::callback("✂️ Удалить несколько истории ✂️", "delete_some_histories");
    let clear_history = InlineKeyboardButton::callback("❌ Очистить историю запросов ❌", "clear_history");

    keyboard.push(vec![check_link]);
    keyboard.push(vec![enter_link]);
    keyboard.push(vec![histories]);
    keyboard.push(vec![delete_some_histories]);
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
    let histories = database::get_histories(user_id.0, None);

    info!("Receiving a request for histories from a user: {}", user_id);

    if histories.iter().count() == 0 {
        bot.send_message(msg.chat.id, "У вас нет истории запросов").await?;
    }
    else {
        let str = create_history_list("Вот ваша история запросов:\n", histories);
        bot.send_message(msg.chat.id, str).await?;
    }

    Ok(())
}

async fn start_deleting_some_histories(bot: Bot, dialogue: MyDialogue, msg: Message, user_id: UserId) -> HandlerResult {
    let histories = database::get_histories(user_id.0, None);

    if histories.iter().count() == 0 {
        bot.send_message(msg.chat.id, "У вас нет истории запросов для удаления").await?;
    }
    else {
        let str = create_history_list("Выберите, какие элементы требуется удалить. Напишите номера элемента через пробел. Вы можете отменить удаление, введя команду /cancel.\n\nИстория запросов:\n", histories);

        bot.send_message(msg.chat.id, str).await?;
        dialogue.update(State::DeletingSomeHistory).await?;
    }

    Ok(())
}

fn create_history_list(str: &str, histories: Vec<History>) -> String {
    let mut str = str.to_string();

    for i in 0..histories.iter().count() {
        let link = format!("\n[{}] {}", i + 1, histories[i].link);
        str.push_str(link.as_str());
    }
    str
}

async fn delete_some_histories(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let numbers_string: String;

    match msg.text() {
        Some(text) => {
            numbers_string = text.to_string();
        }
        None => {
            bot.send_message(msg.chat.id, "Пожалуйста, введите номера элементов.").await?;
            return Ok(());
        }
    }

    let mut numbers: Vec<usize> = Vec::new();

    for number_string in numbers_string.split(" ") {
        let res = number_string.parse::<usize>();

        match res {
            Ok(number) => {
                numbers.push(number - 1);
            }
            Err(_) => {
                bot.send_message(msg.chat.id, "Введенный текст не является корректным!").await?;
                return Ok(());
            }
        }
    }

    let histories = database::get_histories(msg.from().unwrap().id.0, None);
    let numbers_len = numbers.len();
    let mut links: Vec<&str> = Vec::new();

    for number in numbers {
        if number > numbers_len - 1 {
            bot.send_message(msg.chat.id, "Некоторые элементы не существуют в списке!").await?;
            return Ok(());
        }

        links.push(histories[number].link.as_str());
    }

    database::delete_some_histories(msg.from().unwrap().id.0, links);

    bot.send_message(msg.chat.id, "Выбранные вами элементы были удалены").await?;
    dialogue.update(State::Default).await?;

    Ok(())
}

async fn clear_histories(bot: Bot, dialogue: MyDialogue, msg: Message, user_id: UserId) -> HandlerResult {
    database::clear_histories(user_id.0);

    unsafe {
        remove_histories_from_checkers(user_id.0);
    }

    info!("Completely deleted the user's history: {}", user_id);

    bot.send_message(msg.chat.id, "Ваша история запросов успешно очищена!").await?;

    dialogue.update(State::Default).await?;

    Ok(())
}
//