from telegram import Update
from telegram.ext import Updater, CommandHandler, MessageHandler, filters, CallbackContext, CallbackQueryHandler
from telegram import InlineKeyboardMarkup, InlineKeyboardButton
from aiogram import *
import re
TOKEN = 'BOT_TOKEN'

def start(update: Update, context: CallbackContext) -> None:
    user = update.effective_user
    update.message.reply_html(
        fr"Привет {user.mention_html()}!Я ... и я могу проанализировать ваш сайт, то есть проверить скорость его загрузки, "
        fr"и ежечасно приводить отчёт о сбоях в работе указанного вами сайта.",
        reply_markup=main_menu()
    )
    context.user_data.clear()

def main_menu() -> InlineKeyboardMarkup:
    keyboard = [
        [InlineKeyboardButton("Приступим!", callback_data='first_menu')],
        [InlineKeyboardButton("Политика конфиденциальности", callback_data='rules')]
    ]
    return InlineKeyboardMarkup(keyboard)
def first_menu() -> InlineKeyboardMarkup:
    keyboard =[
        [InlineKeyboardButton("Ввести ссылку на сайт")]
        [InlineKeyboardButton("История запросов", callback_data='history')]
        [InlineKeyboardButton("Очистить историю запросов", callback_data='clear')]
    ]
def analyze_text(update, context):
    text = update.message.text
    if re.match(r'https?://\S+', text):  # Проверяем, является ли сообщение ссылкой
        # Здесь можно добавить логику для анализа сайта
        update.message.reply_text(f"Спасибо за ссылку: {text}")
    else:
        update.message.reply_text("Извините, это не похоже на ссылку. Пожалуйста, отправьте ссылку на сайт.")

def main():
    updater = Updater("YOUR_BOT_TOKEN", use_context=True)
    dp = updater.dispatcher

    dp.add_handler(CommandHandler("start", start))
    dp.add_handler(MessageHandler(F.text & ~F.command, analyze_text))

    updater.start_polling()
    updater.idle()


if __name__ == '__main__':
    main()