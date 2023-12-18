![SparkleScannerBot](https://github.com/Onebeld/SparkleScannerBot/assets/44552715/a3b89a34-8fc3-4cac-a887-ef88b35f1241)

# SparkleScannerBot

A Telegram chatbot (codename *N_i_Kit_OS*) that allows you to analyze websites, as well as alert the user if there is a problem with the site.
The bot stores all saved links for hourly checking in its database,
which has `user_id` and `link` columns in the `users` table.

This project was developed as part of the Information Systems Architecture discipline at the university (RSVPU).

The project was created by our team of three people:
- [N-i-Kit-OS](https://github.com/N-i-Kit-OS) — project idea, demo version of the bot in Python, proposed functions (the codename of the project was his idea);
- Onebeld (Dmitry) — creating all bot functions on Rust, documentation writing;
- Beautiful Marina — creating a logo for the bot, creating a presentation, approving a name for the bot, bot testing.

## Functions

- Hourly checking sites for its availability, entered by the user;
- Site Analysis:
  - Displays the site's response code;
  - Displays the site's response time;
  - Checking for robots.txt;
  - Check if sitemap.xml is available;
  - Checks if an SSL certificate exists and, if it does, displays information about it.

## More info

Initially, the Python programming language was used for the bot. However, later we switched to Rust,
as we can use it to create an efficient, secure and fast information system
(We can consider that we used Python to create a demo of the bot).

The bot uses a **sqlite** database to store links. You need to create the database manually, as the bot ***cannot*** create them.

## Getting started

Before you can compile the bot, you must have `rustup` (downloadable from the official Rust website) and Visual Studio with the C++ Application Development component on your system (you can find Visual Studio Build Tools if you don't want to install the IDE).

To compile the bot, use the command in the terminal:
```shell
# Compile the assembly with debug symbols
cargo build

# Alternatively, compile a clean, optimized build of the program
cargo build --release
```

Before running the bot, you must assign environment variables in your operating system.
You can insert this in a batch file for Windows or command files in Unix:
```shell
# Unix-like
export TELOXIDE_TOKEN=<Your token here>
export DATABASE_URL=<Your url>

# Windows command line
set TELOXIDE_TOKEN=<Your token here>
set DATABASE_URL=<Your url>

# Windows PowerShell
$env:TELOXIDE_TOKEN=<Your token here>
$env:DATABASE_URL=<Your url>
```

Then you have to run this bot from a batch (command) file, or run it from the command line. The command line will record the bot's logs.