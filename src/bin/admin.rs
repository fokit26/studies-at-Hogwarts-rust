use std::{
  io::{self, Write},
  net::{IpAddr, SocketAddr},
  process::exit,
  str::FromStr,
  sync::mpsc,
  thread,
};

use clap::Parser;
use hogwarts_guess::{AdminToServer, GuessResult, Message, ServerToAdmin};
use message_io::{
  network::{Endpoint, NetEvent, Transport},
  node,
};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "'Хогвартс Лабораторис' админка")]
#[command(version = "0.1")]
#[command(about = "Позволяет управлять сервером", long_about = None)]
struct Cli {
  #[arg(short, long)]
  address: IpAddr,
  #[arg(short, long, default_value_t = 6969)]
  port: u16,
  #[arg(short = 't', long)]
  auth_token: String,
}

struct State {
  local_addr: SocketAddr,
  server_addr: SocketAddr,
  endpoint: Endpoint,
  auth_token: String,
}

fn main() {
  let cli = Cli::parse();

  let server_addr: SocketAddr = (cli.address, cli.port).into();

  let (hnd, listener) = node::split::<()>();
  let (endpoint, local_addr) = hnd.network().connect(Transport::Tcp, server_addr).unwrap();

  let (notify, wait) = mpsc::channel::<()>();

  let state = &State {
    local_addr,
    server_addr,
    endpoint,
    auth_token: cli.auth_token,
  };

  thread::scope(|s| {
    let handler = hnd.clone();
    s.spawn(move || {
      event_loop(state, listener, handler, notify);
    });

    let handler = hnd.clone();
    s.spawn(move || handle_input(state, handler, wait));
  });
}

fn event_loop(
  state: &State,
  listener: node::NodeListener<()>,
  handler: node::NodeHandler<()>,
  notify: mpsc::Sender<()>,
) {
  listener.for_each(|event| match event.network() {
    NetEvent::Connected(endpoint, is_ok) => {
      if is_ok {
        println!(
          "Подключено: клиент({}) -> эндпоинт({})",
          state.local_addr, endpoint
        );
        println!("Аутефикация...");
        let auth = Message::Ats(AdminToServer::Auth(state.auth_token.clone()));
        handler
          .network()
          .send(endpoint, &bincode::serialize(&auth).unwrap());
      } else {
        println!(
          "Не удалось подключить: клиент({}) -> сервер({})",
          state.local_addr, state.server_addr
        );
        handler.stop();
      }
    }
    NetEvent::Accepted(_, _) => unreachable!(), // Вызывается только с серверной стороны
    NetEvent::Message(_, data) => match bincode::deserialize::<Message>(data) {
      Ok(msg) => {
        if let Message::Sta(sta) = msg {
          handle_message(sta, &notify);
        } else {
          println!("Невалидная категория сообщения!");
        }
      }
      Err(err) => println!("Не удалось распарсить сообщение: {:?}", err),
    },
    NetEvent::Disconnected(_) => {
      println!("Подключение потеряно!");
      handler.stop();
      exit(-2);
    }
  })
}

fn handle_input(state: &State, handler: node::NodeHandler<()>, wait: mpsc::Receiver<()>) -> ! {
  wait.recv().unwrap();
  println!(
    "Добро пожаловать!\
        \n's' - начать эксперимент\
        \n'a' - ответить участнику\
        \n'l' - показать лидерборду\
        \n'w' - показать ожидающих"
  );
  loop {
    let mut inp = String::new();
    print!("> ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut inp).unwrap();
    match inp.trim() {
      "s" => {
        handler.network().send(
          state.endpoint,
          &bincode::serialize(&Message::Ats(AdminToServer::Start)).unwrap(),
        );
      }
      "a" => {
        print!("Уид: ");
        io::stdout().flush().unwrap();
        inp.clear();
        io::stdin().read_line(&mut inp).unwrap();
        let uuid = match Uuid::from_str(inp.trim()) {
          Err(err) => {
            println!("Ошибка: {}", err);
            continue;
          }
          Ok(res) => res,
        };
        print!("Ответ (<, >, =): ");
        io::stdout().flush().unwrap();
        inp.clear();
        io::stdin().read_line(&mut inp).unwrap();
        let ans = match inp.trim() {
          "<" => GuessResult::Less,
          ">" => GuessResult::More,
          "=" => GuessResult::Equal,
          _ => {
            println!("Невалидный символ!");
            continue;
          }
        };
        handler.network().send(
          state.endpoint,
          &bincode::serialize(&Message::Ats(AdminToServer::SendAnswer {
            target: uuid,
            answer: ans,
          }))
          .unwrap(),
        );
      }
      "l" => {
        handler.network().send(
          state.endpoint,
          &bincode::serialize(&Message::Ats(AdminToServer::Stats)).unwrap(),
        );
      }
      "w" => {
        handler.network().send(
          state.endpoint,
          &bincode::serialize(&Message::Ats(AdminToServer::WaitAnswers)).unwrap(),
        );
      }
      _ => println!("Некорректная комманда!"),
    }
  }
}

fn handle_message(message: ServerToAdmin, notify: &mpsc::Sender<()>) {
  match message {
    ServerToAdmin::Stats(stats) => {
      let mut vec: Vec<_> = stats.0.into_iter().collect();
      vec.sort_unstable_by_key(|e| e.1);
      println!("\nСтатистика (уид, количество):");
      for (uuid, count) in vec.iter().rev() {
        println!("{} :: {}", uuid, count);
      }
      print!("> ");
      io::stdout().flush().unwrap();
    }
    ServerToAdmin::WaitAnswers(wait_answers) => {
      println!("\nОжидающие ответа (уид, предположение)");
      for (uuid, guess) in wait_answers.0 {
        println!("{} :: {}", uuid, guess);
      }
      print!("> ");
      io::stdout().flush().unwrap();
    }
    ServerToAdmin::ResultAuth(ack) => {
      if ack {
        println!("Аутефикация успешна!");
        notify.send(()).unwrap();
      } else {
        println!("Аутефикация неуспешна!");
        exit(-1);
      }
    }
  }
}
