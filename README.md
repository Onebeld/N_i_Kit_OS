# N_i_Kit_OS

A project developed within the university (RSVPU). This is a Telegram chatbot that allows you to check sites for traffic, security, transmission and so on, written in Rust.

Originally we wanted to write in Python, but I got bored, and we decided to do it in Rust. In general, we made the code in Python as a demo.

The project was made in collaboration with [N-i-Kit-OS](https://github.com/N-i-Kit-OS), my fellow student (in fact, the code name of the project was his idea).

The bot uses a **sqlite** database to store links

## Getting started

First, you must create environment variables on your system:
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

You can then launch the bot from the command line. Exactly from it, since logs will be displayed on the screen.