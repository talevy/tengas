#![allow(unused_must_use, dead_code, unused_variables)]
#![feature(fnbox)]

extern crate mio;
extern crate hyper;

mod http;

use http::HttpStream; 

use std::thread;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::io;
use std::io::Read;
use std::boxed::FnBox;

use mio::{Sender, EventSet, PollOpt, Token, EventLoop, Handler};
use mio::util::Slab;
use mio::tcp::TcpSocket;


use hyper::http::h1::Http11Message;
use hyper::Url;
use hyper::client::Response;
use hyper::header::{self, Headers};
use hyper::http::message::{HttpMessage, RequestHead};
use hyper::method::Method;


type Callback = Box<FnBox(String) + Send + 'static>;
type RequestAndCallback = (Request, Callback);

struct Request {
    url: Url
}

struct Connection {
    url: Url,
    token: Token,
    stream: Box<HttpStream>,
    callback: Callback
}

struct ResponseManager {
    conns: Slab<Connection>,
}

impl ResponseManager {
    fn new() -> ResponseManager {
        ResponseManager {
            conns: Slab::new_starting_at(Token(0), 128)
        }
    } 

    fn find_connection_by_token<'a>(&'a mut self, token: Token) -> &'a mut Connection {
        &mut self.conns[token]
    }

    fn remove(&mut self, token: Token) -> Option<Connection> {
        self.conns.remove(token)
    }
}

impl Handler for ResponseManager {
    type Timeout = usize;
    type Message = RequestAndCallback;

    fn ready(&mut self, event_loop: &mut EventLoop<ResponseManager>, token: Token, event: EventSet) {
        if event.is_hup() {
            event_loop.shutdown();
            return;
        }

        if event.is_readable() {
            let mut body = String::new();

            let connection = self.remove(token).unwrap();
            let mut resp = Response::new(connection.url.clone(), connection.stream.clone()).ok().unwrap();
            let _  = resp.read_to_string(&mut body);
            let cb = connection.callback;
            cb(body);
        }

        if event.is_writable() {
            let connection = self.find_connection_by_token(token);
            let stream = connection.stream.clone();

            // parse url to host,port,path
            let url = connection.url.clone();

            let mut headers = Headers::new();
            headers.set( header::Host {
                hostname: url.domain().map_or(String::new(), |x| x.to_string()),
                port: url.port()
            });

            headers.set(header::Connection::keep_alive());

            let mut msg = Http11Message::with_stream(stream);

            let req_head = RequestHead {
                headers: headers,
                method: Method::Get,
                url: url
            };

            msg.set_outgoing(req_head);
            msg.flush_outgoing();
        }
    }

    fn notify(&mut self, event_loop: &mut EventLoop<ResponseManager>, msg: RequestAndCallback) {
        let (request, callback) = msg;

        if request.url.domain().unwrap_or("shutdown") == "shutdown" {
            event_loop.shutdown();
            return;
        }

        match self.conns.insert_with(|token| {
            // hack because mio::TcpStream does not implement ToSocketAddrs (private in std)
            let url = request.url.clone();
            let addr_str = (url.domain().unwrap_or("localhost"), url.port().unwrap_or(80));
            let addr = addr_str.to_socket_addrs().ok().map(|x| { x.into_iter().next().unwrap() }).unwrap();
            let (sock, _) = TcpSocket::v4().unwrap().connect(&addr).unwrap();
            let boxed_sock = Box::new(HttpStream(sock));

            Connection {
                url: request.url.clone(),
                token: token,
                stream: boxed_sock,
                callback: callback
            }
        }) {
            Some(token) => {
                let conn = self.find_connection_by_token(token);
                event_loop.register_opt(&conn.stream.0, token, EventSet::writable() | EventSet::readable(), PollOpt::edge() | PollOpt::oneshot()).unwrap();
            },
            None => {
                event_loop.shutdown();
            }
        }
    }
}

struct Client {
    sender: Arc<Sender<RequestAndCallback>>
}

impl Client {

    fn new() -> io::Result<Client> {
        let mut event_loop = try!(EventLoop::new());
        let chan = Arc::new(event_loop.channel());

        thread::spawn(move || {
            event_loop.run(&mut ResponseManager::new());
        });

        Ok(Client {
            sender: chan
        })
    }

    fn get(&mut self, surl: &str, callback: Callback) {
        let url = Url::parse(surl).ok().expect("failed parsing surl");

        let r = Request {
            url: url
        };

        self.sender.send((r, callback));
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // TODO: wait for event_loop to stop running before exiting
        let f = Box::new(move |r: String| { });
        self.sender.send((Request { url: Url::parse("http://shutdown/").ok().expect("FD") }, f));
    }
}

#[test]
fn it_works() {
    let mut client = Client::new().ok().expect("unable to start client");

    for _ in 0..10 {
        let cb = Box::new(move|response: String| { println!("{}", response) });
        client.get("http://httpbin.org/get", cb)
    }

    thread::sleep_ms(1000);
}
