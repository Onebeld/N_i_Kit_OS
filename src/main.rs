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
use crate::database::Links;

extern crate pretty_env_logger;
#[macro_use] extern crate log;

mod database;
mod website;

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
type MyDialogue = Dialogue<State, InMemStorage<State>>;

const SECONDS: u64 = 3600;

const STICKER_WELCOME: &str = "CAACAgIAAxkBAAEne6RlSyQM7sJfMXWBN3u-dfEgIlxzoAACBQADwDZPE_lqX5qCa011MwQ";
const STICKER_ERROR: &str = "CAACAgIAAxkBAAEne6JlSyP9VdH3N8Mk2imfp7BgFRu9NwACEAADwDZPE-qBiinxHwLoMwQ";

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Поддерживаются следующие команды:")]
enum Command {
    #[command(description = "запускает процедуру.")]
    Start,
    #[command(description = "показать меню действий бота.")]
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
    ReceiveConfirmRemoveLinks,
    DeletingSomeLinks
}

#[tokio::main]
async fn main() -> HandlerResult {
    pretty_env_logger::formatted_timed_builder().filter_level(LevelFilter::Info).init();

    let bot = Bot::from_env();

    info!("The bot is up and running and ready to go!");

    launch_checkers(bot.clone());

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
fn launch_checkers(bot: Bot) {
    let mut interval = tokio::time::interval(Duration::from_secs(SECONDS));

    info!("A thread has been launched to test sites.");

    tokio::spawn(async move {
        loop {
            interval.tick().await;

            info!("Runs a site checker");

            let all_links = database::get_all_links();

            for one_link in all_links {
                let user_id: UserId = UserId(one_link.user_id as u64);
                let status_code = website::get_request_code(&one_link.link).await;

                match status_code {
                    Ok(status_code_unwrapped) => {
                        handle_status_code(&bot, one_link.user_id as u64, one_link.link, status_code_unwrapped).await;
                    }
                    Err(err) => {
                        error!("Failed to verify the site for the user: {}. Description: {}", one_link.user_id, err.to_string());
                        let _ = bot.send_message(user_id, format!("Не удалось проверить сайт по ссылке: {}", one_link.link)).await;
                    }
                }
            }
        }
    });
}

/// Processes the site status code and sends a message to the user if there are any problems
///
/// # Arguments
///
/// * `bot`: A bot instance
/// * `user_id`: User ID in Telegram
/// * `link`: Site link
/// * `status_code`: Server status code
async fn handle_status_code(bot: &Bot, user_id: u64, link: String, status_code: u16) -> HandlerResult {
    let mut text = format!("Произошла ошибка при проверки ссылки: {link}");

    match status_code {
        403 => {
            text = format!("{text}\n\nКод ошибки: {status_code}\nБот не может получить доступ");
        }
        404 => {
            text = format!("{text}\n\nКод ошибки: {status_code}\nЭтой страницы не существует");
        }
        500 => {
            text = format!("{text}\n\nКод ошибки: {status_code}\nВнутренняя ошибка сервера");
        }
        503 => {
            text = format!("{text}\n\nКод ошибки: {status_code}\nСервис недоступен");
        }

        200..=299 | 300..=399 => { return Ok(()); }
        _ => {
            text = format!("{text}\n\nКод ошибки: {status_code}");
        }
    }

    bot.send_sticker(UserId(user_id), InputFile::file_id(STICKER_ERROR)).await?;
    bot.send_message(UserId(user_id), text).await?;

    Ok(())
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![State::Default]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Menu].endpoint(show_actions))
            .branch(case![Command::Help].endpoint(help)))
        .branch(case![State::ReceiveLink]
            .branch(case![Command::Cancel].endpoint(cancel_receive_link)))
        .branch(case![State::DeletingSomeLinks]
            .branch(case![Command::Cancel].endpoint(cancel_deleting_some_links)))
        .branch(case![State::ReceiveLinkForChecking]
            .branch(case![Command::Cancel].endpoint(cancel_receive_link)));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::ReceiveLink].endpoint(receive_link))
        .branch(case![State::DeletingSomeLinks].endpoint(delete_some_links))
        .branch(case![State::ReceiveLinkForChecking].endpoint(check_site));

    let callback_query_handler = Update::filter_callback_query()
        .branch(case![State::Default].endpoint(menu_choice_callback_handler))
        .branch(case![State::ReceiveConfirmRemoveLinks].endpoint(menu_confirm_remove_links_callback_handler));

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
    info!("A new user has joined the bot: {}", msg.from().expect("Unable to determine user ID").id);

    let text = format!("🚀 Привет, {}! Я - {}, и я могу проанализировать Ваш сайт, то есть проверить скорость его загрузки и ежечасно приводить отчёт о сбоях в работе указанного Вами сайта.", msg.from().expect("Unable to define a user name").first_name, bot.get_me().await?.first_name);
    let keyboard = create_beginning_menu_keyboard().await;

    bot.send_sticker(msg.chat.id, InputFile::file_id(STICKER_WELCOME)).await?;
    bot.send_message(msg.chat.id, text).reply_markup(keyboard).await?;

    Ok(())
}

/// Sends a message to the user that displays all bot commands
///
/// # Arguments
///
/// * `bot`: Bot instance
/// * `msg`: Message sent by the user
async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
    Ok(())
}

/// Event handler after a user clicks a button in the default state, which activates a bot action
/// depending on the selection
///
/// # Arguments
///
/// * `bot`: Bot instance
/// * `dialogue`: A handle for controlling dialogue state
/// * `q`: Response from the user after pressing the button
async fn menu_choice_callback_handler(bot: Bot, dialogue: MyDialogue, q: CallbackQuery) -> HandlerResult {
    if let Some(data) = &q.data {
        if let Some(message) = q.clone().message {
            match data.as_str() {
                "begin" => show_actions(bot, message).await?,
                "check_link" => start_check_link(bot, dialogue, message).await?,
                "get_links" => get_all_links_from_user(bot, q).await?,
                "clear_all_links" => ask_about_clear_links(bot, dialogue, message).await?,
                "delete_some_links" => start_deleting_some_links(bot, dialogue, q).await?,

                "enter_link" => start_enter_link(bot, dialogue, message).await?,

                _ => (),
            }
        }
    }

    Ok(())
}

/// Event handler after the user clicks the button in the ReceiveConfirmRemoveHistories state,
/// which determines whether to delete the history or not
///
/// # Arguments
///
/// * `bot`: Bot instance
/// * `dialogue`: A handle for controlling dialogue state
/// * `q`: Response from the user after pressing the button
async fn menu_confirm_remove_links_callback_handler(bot: Bot, dialogue: MyDialogue, q: CallbackQuery) -> HandlerResult {
    if let Some(data) = &q.data {
        if let Some(message) = &q.message {
            match data.as_str() {
                "confirm" => clear_links(bot, dialogue, q).await?,
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

/// Sends a message to the user that displays the bot's action menu
///
/// # Arguments
///
/// * `bot`: Bot instance
/// * `msg`: Message sent by the user
async fn show_actions(bot: Bot, msg: Message) -> HandlerResult {
    let text = format!("{} к вашим услугам!\nЧто вы хотите сделать?", bot.get_me().await?.first_name);
    let keyboard = create_main_menu_keyboard().await;

    bot.send_message(msg.chat.id, text).reply_markup(keyboard).await?;

    Ok(())
}

/// Sends a message to the user asking for confirmation to clear the history
///
/// # Arguments
///
/// * `bot`: Bot instance
/// * `dialogue`: A handle for controlling dialogue state
/// * `msg`: Message sent by the user
async fn ask_about_clear_links(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let text = "❓ Вы действительно хотите очистить все сохраненные вами ссылки? ❓";
    let keyboard = create_confirmation_menu_keyboard().await;

    bot.send_message(msg.chat.id, text).reply_markup(keyboard).await?;

    dialogue.update(State::ReceiveConfirmRemoveLinks).await?;

    Ok(())
}


/// Sends a message to the user telling them to enter a link and also goes to the ReceiveLink state
///
/// # Arguments
///
/// * `bot`: Bot instance
/// * `dialogue`: A handle for controlling dialogue state
/// * `msg`: Message sent by the user
async fn start_enter_link(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Пожалуйста, введите ссылку. Для отмены ввода ссылки введите команду /cancel").await?;
    dialogue.update(State::ReceiveLink).await?;

    Ok(())
}

/// Sends a message to the user that a link needs to be entered, and enters the
/// ReceiveLinkForChecking state
///
/// # Arguments
///
/// * `bot`: Bot instance
/// * `dialogue`: A handle for controlling dialogue state
/// * `msg`: Message sent by the user
async fn start_check_link(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Пожалуйста, введите ссылку. Для отмены ввода ссылки введите команду /cancel").await?;
    dialogue.update(State::ReceiveLinkForChecking).await?;

    Ok(())
}

async fn receive_link(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let user_id = msg.from().expect("Unable to determine user ID").id;

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

    if !website::has_http_s(&url) {
        url = format!("https://{}", url);
    }

    if is_url(&url) {
        if database::is_link_exists(user_id.0, &url) {
            bot.send_message(msg.chat.id, "Данная ссылка уже была добавлена").await?;

            dialogue.update(State::Default).await?;

            return Ok(());
        }

        database::add_link(user_id.0, &url);

        info!("Added a new link to the database from the user: {}", user_id);

        bot.send_message(msg.chat.id, "Спасибо за ссылку! Теперь я буду проверять эту ссылку каждый час").await?;

        dialogue.update(State::Default).await?;
    }
    else {
        bot.send_message(msg.chat.id, "Данный текст не является ссылкой!").await?;
    }

    Ok(())
}

/// Checks the site received from the user and sends the information to the user
///
/// # Arguments
///
/// * `bot`: Bot instance
/// * `dialogue`: A handle for controlling dialogue state
/// * `msg`: Message sent by the user
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

    if !website::has_http_s(&url) {
        url = format!("https://{}", url);
    }

    if is_url(&url) {
        let sent_message = bot.send_message(msg.chat.id, "Пожалуйста, подождите...").await?;

        info!("Site information for the user is requested: {}", msg.from().expect("Unable to determine user ID").id.0);

        let site_information = website::get_site_information(&url).await?;

        let mut text = format!("❔ Информация о введеном вами сайте ❔\n\n");

        text = format!("{text}📝 Код ответа: {}\n", site_information.status_code);
        text = format!("{text}🕔 Время ответа: {} милисекунд\n", site_information.duration);

        match site_information.has_robots {
            200 => {
                text = format!("{text}🤖 Наличие robots.txt: есть\n")
            }
            _ => {
                text = format!("{text}🤖 Наличие robots.txt: нет (код ответа: {})\n", site_information.has_robots);
            }
        }
        match site_information.has_sitemap {
            200 => {
                text = format!("{text}🗺 Наличие sitemap.xml: есть\n\n")
            }
            _ => {
                text = format!("{text}🗺 Наличие sitemap.xml: нет (код ответа: {})\n\n", site_information.has_sitemap);
            }
        }

        match site_information.certificate {
            Some(cert) => {
                text = format!("{text}📄 Сертификат:\
                \nОбщее название: {}\
                \nОрганизация: {}\
                \nСтрана: {}\
                \nИздатель: {}\
                \nВремя окончания: {}\n\n", cert.intermediate.common_name,
                               cert.intermediate.organization,
                               cert.intermediate.country,
                               cert.intermediate.issuer,
                               cert.intermediate.time_to_expiration);
            }
            None => {
                text = format!("{text}📄 Сертификат: не найден\n\n")
            }
        }

        bot.edit_message_text(msg.chat.id, sent_message.id, text).await?;

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

async fn cancel_deleting_some_links(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Вы отменили удаление ссылок").await?;

    dialogue.update(State::Default).await?;

    Ok(())
}

// Creating bot menus
/// Creates a menu with a single button to start working with the bot
async fn create_beginning_menu_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    let begin = InlineKeyboardButton::callback("Приступим!", "begin");

    keyboard.push(vec![begin]);

    InlineKeyboardMarkup::new(keyboard)
}

/// Creates a bot action menu
async fn create_main_menu_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    let check_link = InlineKeyboardButton::callback("🔭 Проанализировать сайт 🔭", "check_link");
    let enter_link = InlineKeyboardButton::callback("✏️ Ввести ссылку на сайт ✏️", "enter_link");
    let links = InlineKeyboardButton::callback("📒 Получить все ссылки 📒", "get_links");
    let delete_some_histories = InlineKeyboardButton::callback("✂️ Удалить несколько ссылок ✂️", "delete_some_links");
    let clear_all_links = InlineKeyboardButton::callback("❌ Очистить все ссылки ❌", "clear_all_links");

    keyboard.push(vec![check_link]);
    keyboard.push(vec![enter_link]);
    keyboard.push(vec![links]);
    keyboard.push(vec![delete_some_histories]);
    keyboard.push(vec![clear_all_links]);

    InlineKeyboardMarkup::new(keyboard)
}

/// Creates a menu where you must confirm the action of clearing all history
async fn create_confirmation_menu_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    let confirmation = InlineKeyboardButton::callback("Очистить", "confirm");
    let cancel = InlineKeyboardButton::callback("Отмена", "cancel");

    keyboard.push(vec![confirmation]);
    keyboard.push(vec![cancel]);

    InlineKeyboardMarkup::new(keyboard)
}
//

/// Retrieves all saved links by user ID and sends a message with the result
///
/// # Arguments
///
/// * `bot`: Bot instance
/// * `q`: Response from the user after pressing the button
async fn get_all_links_from_user(bot: Bot, q: CallbackQuery) -> HandlerResult {
    let user_id = q.from.id;
    let histories = database::get_all_links_from_user(user_id.0, None);

    info!("Receiving a request for all links from the user: {}", user_id);

    if histories.iter().count() == 0 {
        bot.send_message(user_id, "У вас нет сохраненных ссылок").await?;
    }
    else {
        let str = create_links_list("Вот ваши сохраненные ссылки:\n", histories);
        bot.send_message(user_id, str).await?;
    }

    Ok(())
}

///
///
/// # Arguments
///
/// * `bot`: Bot instance
/// * `dialogue`: A handle for controlling dialogue state
/// * `q`: Response from the user after pressing the button
async fn start_deleting_some_links(bot: Bot, dialogue: MyDialogue, q: CallbackQuery) -> HandlerResult {
    let user_id = q.from.id;
    let histories = database::get_all_links_from_user(user_id.0, None);

    if histories.iter().count() == 0 {
        bot.send_message(user_id, "У вас нет истории запросов для удаления").await?;
    }
    else {
        let str = create_links_list("Выберите, какие элементы требуется удалить. Напишите номера элемента через пробел. Вы можете отменить удаление, введя команду /cancel.\n\nИстория запросов:\n", histories);

        bot.send_message(user_id, str).await?;
        dialogue.update(State::DeletingSomeLinks).await?;
    }

    Ok(())
}

/// Creates a formatted list of links
///
/// # Arguments
///
/// * `str`: Message to user
/// * `histories`: List of links
///
/// returns: Message to user with a formatted list of links
fn create_links_list(str: &str, links: Vec<Links>) -> String {
    let mut str = str.to_string();

    for i in 0..links.iter().count() {
        let link = format!("\n[{}] {}", i + 1, links[i].link);
        str.push_str(&link);
    }
    str
}

/// Removes some references from the database that the user enters and returns the result of
/// the operation
///
/// # Arguments
///
/// * `bot`: Bot instance
/// * `dialogue`: A handle for controlling dialogue state
/// * `msg`: Message sent by the user
async fn delete_some_links(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let numbers_string: String;
    let user_id = msg.from().expect("Unable to determine user ID").id.0;

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

    let histories = database::get_all_links_from_user(user_id, None);
    let numbers_len = numbers.len();
    let mut links: Vec<&str> = Vec::new();

    for number in numbers {
        if number > numbers_len - 1 {
            bot.send_message(msg.chat.id, "Некоторые элементы не существуют в списке!").await?;
            return Ok(());
        }

        links.push(histories[number].link.as_str());
    }

    database::delete_some_histories(user_id, links);

    bot.send_message(msg.chat.id, "Выбранные вами элементы были удалены").await?;
    dialogue.update(State::Default).await?;

    Ok(())
}

/// Removes all references from the database from the user and returns the result of the operation
///
/// # Arguments
///
/// * `bot`: Bot instance
/// * `dialogue`: A handle for controlling dialogue state
/// * `q`: Response from the user after pressing the button
async fn clear_links(bot: Bot, dialogue: MyDialogue, q: CallbackQuery) -> HandlerResult {
    let user_id = q.from.id;

    database::clear_all_links(user_id.0);

    info!("Completely deleted the user's history: {}", user_id);

    bot.edit_message_text(user_id, q.message.unwrap().id, "Ваша история запросов успешно очищена!").await?;

    dialogue.update(State::Default).await?;

    Ok(())
}