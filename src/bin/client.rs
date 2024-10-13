use std::{
  net::{IpAddr, SocketAddr},
  sync::mpsc,
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

fn handle_message(
  message: ServerToClient,
  notify: &mpsc::Sender<()>,
  handler: &NodeHandler<()>,
  endpoint: Endpoint,
) {
  match message {
    ServerToClient::RegisterUUID(uuid) => {
      println!("Токен участника: {}", uuid);
    }
    ServerToClient::ExperimentStart(uuid) => {
      println!("Начало эксперимента!");
      notify.send(()).unwrap();
      handler.network().send(
        endpoint,
        &bincode::serialize(&Message::Cts(ClientToServer::Ack(uuid))).unwrap(),
      );
    }
    ServerToClient::Answer(guess_result, uuid) => {
      print!("Результаты попытки: ");
      match guess_result {
        GuessResult::Equal => println!("равно"),
        GuessResult::Less => println!("меньше"),
        GuessResult::More => println!("больше"),
      }
      handler.network().send(
        endpoint,
        &bincode::serialize(&Message::Cts(ClientToServer::Ack(uuid))).unwrap(),
      );
    }
  }
}

fn main() {
  let cli = Cli::parse();

  let server_addr: SocketAddr = (cli.address, cli.port).into();

  let (handler, listener) = node::split::<()>();
  let (endpoint, local_addr) = handler
    .network()
    .connect(Transport::Tcp, server_addr.clone())
    .unwrap();

  let (notify, _wait) = mpsc::channel::<()>();

  listener.for_each(|event| match event.network() {
    NetEvent::Connected(endpoint, is_ok) => {
      if is_ok {
        println!(
          "Подключено: клиент({}) -> эндпоинт({})",
          local_addr, endpoint
        );
        println!("Аутефикация...");
        let auth = Message::Cts(ClientToServer::Register);
        handler
          .network()
          .send(endpoint, &bincode::serialize(&auth).unwrap());
      } else {
        println!(
          "Не удалось подключить: клиент({}) -> сервер({})",
          local_addr, server_addr
        );
        handler.stop();
      }
    }
    NetEvent::Accepted(_, _) => unreachable!(), // Вызывается только с серверной стороны
    NetEvent::Message(_, data) => match bincode::deserialize::<Message>(data) {
      Ok(msg) => {
        if let Message::Stc(sta) = msg {
          handle_message(sta, &notify, &handler, endpoint.clone());
        } else {
          println!("Невалидная категория сообщения!");
        }
      }
      Err(err) => println!("Не удалось распарсить сообщение: {:?}", err),
    },
    NetEvent::Disconnected(_) => {
      println!("Подключение потеряно!");
      handler.stop();
    }
  });
}
