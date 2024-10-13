#![feature(iter_intersperse)]

use std::{
  io::{self, Write},
  net::{IpAddr, SocketAddr},
  process::exit,
  sync::mpsc,
  thread,
};

use clap::Parser;
use hogwarts_guess::{ClientToServer, GuessResult, Message, ServerToClient};
use message_io::{
  network::{Endpoint, NetEvent, Transport},
  node::{self, NodeHandler},
};

#[derive(Parser)]
#[command(name = "'Хогвартс Лабораторис' клиент")]
#[command(version = "0.1")]
#[command(about = "Позволяет участвовать в эксперименте о угадывании чисел", long_about = None)]
struct Cli {
  #[arg(short, long)]
  address: IpAddr,
  #[arg(short, long, default_value_t = 6969)]
  port: u16,
}

struct State {
  local_addr: SocketAddr,
  server_addr: SocketAddr,
  endpoint: Endpoint,
}

fn main() {
  let cli = Cli::parse();

  let server_addr: SocketAddr = (cli.address, cli.port).into();

  let (hnd, listener) = node::split::<()>();
  let (endpoint, local_addr) = hnd.network().connect(Transport::Tcp, server_addr).unwrap();

  let (notify, wait) = mpsc::channel::<()>();

  let state = &State {
    endpoint,
    local_addr,
    server_addr,
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
  handler: NodeHandler<()>,
  notify: mpsc::Sender<()>,
) {
  listener.for_each(|event| match event.network() {
    NetEvent::Connected(endpoint, is_ok) => {
      if is_ok {
        println!(
          "Подключено: клиент({}) -> эндпоинт({})",
          state.local_addr, state.endpoint
        );
        println!("Аутефикация...");
        let auth = Message::Cts(ClientToServer::Register);
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
        if let Message::Stc(sta) = msg {
          handle_message(state, sta, &handler, &notify);
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
  });
}

fn handle_message(
  state: &State,
  message: ServerToClient,
  handler: &NodeHandler<()>,
  notify: &mpsc::Sender<()>,
) {
  match message {
    ServerToClient::RegisterUUID(uuid) => {
      println!("Токен участника: {}", uuid);
    }
    ServerToClient::ExperimentStart(uuid) => {
      println!("Начало эксперимента!");
      notify.send(()).unwrap();
      handler.network().send(
        state.endpoint,
        &bincode::serialize(&Message::Cts(ClientToServer::Ack(uuid))).unwrap(),
      );
    }
    ServerToClient::Answer(guess_result, uuid) => {
      print!("\nРезультаты попытки: ");
      match guess_result {
        GuessResult::Equal => println!("равно"),
        GuessResult::Less => {
          println!("меньше");
          notify.send(()).unwrap();
        }
        GuessResult::More => {
          println!("больше");
          notify.send(()).unwrap();
        }
      }
      handler.network().send(
        state.endpoint,
        &bincode::serialize(&Message::Cts(ClientToServer::Ack(uuid))).unwrap(),
      );
      print!("> ");
      io::stdout().flush().unwrap();
    }
  }
}

fn handle_input(state: &State, handler: node::NodeHandler<()>, wait: mpsc::Receiver<()>) -> ! {
  wait.recv().unwrap();
  println!(
    "Добро пожаловать!\
        \n'g' - отправить предположение\
        \n'h' - посмотреть историю ответов"
  );
  let mut history = Vec::new();
  loop {
    let mut inp = String::new();
    print!("> ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut inp).unwrap();
    match inp.trim() {
      "g" => {
        print!("Предположение: ");
        io::stdout().flush().unwrap();
        inp.clear();
        io::stdin().read_line(&mut inp).unwrap();
        let guess = match inp.trim().parse() {
          Err(err) => {
            println!("Ошибка: {}", err);
            continue;
          }
          Ok(res) => res,
        };
        history.push(guess);
        handler.network().send(
          state.endpoint,
          &bincode::serialize(&Message::Cts(ClientToServer::Guess(guess))).unwrap(),
        );
      }
      "h" => {
        println!("История:");
        for line in history.chunks(5) {
          print!("{}", line[0]);
          for e in line.iter().skip(1) {
            print!(", {}", e);
          }
          println!();
        }
      }
      _ => println!("Некорректная комманда!"),
    }
  }
}
