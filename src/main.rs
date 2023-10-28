use teloxide::{
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Me},
    prelude::*,
    Bot,
    utils::command::BotCommands
};
use crate::database::Database;

mod database;
mod website_checker;

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;

#[derive(BotCommands)]
#[command(rename_rule = "lowercase")]
enum Command {
    Start,
    Menu
}

#[tokio::main]
async fn main() -> HandlerResult {
    let bot = Bot::from_env();

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler));

    Dispatcher::builder(bot, handler).enable_ctrlc_handler().build().dispatch().await;

    Ok(())
}

async fn message_handler(bot: Bot, msg: Message, me: Me) -> HandlerResult {
    if let Some(text) = msg.text() {
        match Command::parse(text, me.username()) {
            Ok(Command::Start) => start(bot, msg).await?,
            Ok(Command::Menu) => show_actions(bot, msg).await?,

            Err(_) => {
                bot.send_message(msg.chat.id, "Команда не найдена!").await?;
            }
        }
    }

    Ok(())
}

async fn callback_handler(bot: Bot, q: CallbackQuery) -> HandlerResult {
    if let Some(data) = q.data{
        if let Some(message) = q.message {
            match data.as_str() {
                "begin" => show_actions(bot, message).await?,
                "get_history" => get_histories(bot, message).await?,
                "clear_history" => ask_about_clear_histories(bot, message).await?,
                "confirm_clear_history" => clear_histories(bot, message).await?,
                _ => (),
            }
        }
    }

    Ok(())
}

async fn start(bot: Bot, msg: Message) -> HandlerResult {
    let text = format!("Привет, {}! Я - {}, и я могу проанализировать Ваш сайт, то есть проверить скорость его загрузки и ежечасно приводить отчёт о сбоях в работе указанного Вами сайта.", msg.from().unwrap().first_name, bot.get_me().await?.first_name);
    let keyboard = create_beginning_menu_keyboard().await;

    bot.send_message(msg.chat.id, text).reply_markup(keyboard).await?;

    Ok(())
}

async fn show_actions(bot: Bot, msg: Message) -> HandlerResult {
    let text = "Что вы хотите сделать?";
    let keyboard = create_main_menu_keyboard().await;

    bot.send_message(msg.chat.id, text).reply_markup(keyboard).await?;

    Ok(())
}

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

async fn get_histories(bot: Bot, msg: Message) -> HandlerResult {
    let histories = Database::get_histories(msg.from().unwrap().id.0 as f64, None);

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

async fn ask_about_clear_histories(bot: Bot, msg: Message) -> HandlerResult {
    let text = "Вы действительно хотите очистить истоирю запросов? Нажмите на кнопку ниже, если вы подтверждаете своё действие.";
    let keyboard = create_confirmation_menu_keyboard().await;

    bot.send_message(msg.chat.id, text).reply_markup(keyboard).await?;

    Ok(())
}

async fn clear_histories(bot: Bot, msg: Message) -> HandlerResult {
    Database::clear_histories(msg.from().unwrap().id.0 as f64);

    bot.send_message(msg.chat.id, "Ваша история запросов успешно очищена!").await?;

    Ok(())
}