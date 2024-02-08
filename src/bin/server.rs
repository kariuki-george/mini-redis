use std::{collections::VecDeque, ops::Sub};

use mini_redis::{
    connection::{Connection, ConnectionError},
    db::DB,
    frame::{Frame, FrameError},
    rdb::RDB,
    runner::{Runner, RunnerError},
};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to read .env file");

    let subscriber = tracing_subscriber::fmt()
        .compact()
        // Display source code file paths
        .with_file(true)
        // Display source code line numbers
        .with_line_number(true)
        // Display the thread ID an event was recorded on
        .with_thread_ids(true)
        // Don't display the event's target (module path)
        .with_target(false)
        // Build the subscriber
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    tracing::info!("MINIREDIS: Starting mini-redis");
    // Start db
    let db = DB::new();
    // Start rdb instance
    let mut rdb = RDB::new(db.clone());

    // Load saved entries into db
    rdb.load().await.unwrap();

    // Open TCP listener for new connections
    let addr = std::env::var("ADDR").expect("ADDR env var not provided");
    let listener = TcpListener::bind(addr.clone()).await.unwrap();
    tracing::info!("MINIREDIS: Listening for connections at {}", addr);

    loop {
        /*
        Block the main thread until a connection is created.
        Clone the db for that connection to use.
        Create a new tokio handle(green thread) to handle the connection
         */
        let (stream, _) = listener.accept().await.unwrap();
        let start = chrono::Utc::now();
        let mut db = db.clone();

        let handle = tokio::spawn(async move {
            let mut connection = Connection::new(stream);

            // Get a full frame from the connection
            // A frame in this case refers to a complete data unit in this case corresponds to the redis protocol spec
            // Most frames are of either Array or String
            // E.g [Frame::SimpleString("SET"),Frame::SimpleString("KEY"),Frame::SimpleString("VALUE")]
            // E.g Frame::SimpleString("PING")

            let frame = match connection.read_frame().await {
                Ok(frame) => match frame {
                    Some(frame) => frame,
                    None => Frame::Array(VecDeque::new()),
                },
                Err(err) => {
                    handle_err(err, &mut connection).await;
                    return;
                }
            };

            // Takes a frame from earlier step and executes it.
            // Takes the db instance for frames that require db access.

            let mut runner = Runner::new(&mut db);
            let results = runner.run(frame);

            // Parse the results from the runner
            // If successful, write the resulting frame back to the client
            match results {
                Err(err) => handle_runner_err(err, &mut connection).await,
                Ok(frame) => connection.write_all(frame).await.unwrap(),
            }
            let stop = chrono::Utc::now();

            // Shutdown the connection
            connection.shutdown().await;
            let total_time = stop.sub(start).to_std().unwrap().as_nanos();
            tracing::info!("MINIREDIS: Handled request for {} ns", total_time)
        });

        handle.await.unwrap();
    }
}

async fn handle_err(connection_error: ConnectionError, connection: &mut Connection) {
    match connection_error {
        ConnectionError::FrameError(err) => match err {
            FrameError::Other(err) => connection.write_all(Frame::SimpleError(err)).await.unwrap(),
            FrameError::Incomplete => todo!(),
        },
        ConnectionError::IOError(err) => connection
            .write_all(Frame::SimpleError(format!("{}", err)))
            .await
            .unwrap(),
    }
}

async fn handle_runner_err(runner_error: RunnerError, connection: &mut Connection) {
    match runner_error {
        RunnerError::Other(err) => connection.write_all(Frame::SimpleError(err)).await.unwrap(),
        RunnerError::Incomplete => connection
            .write_all(Frame::SimpleError(
                "Protocol Error: Incorrect usage of command".to_string(),
            ))
            .await
            .unwrap(),
        RunnerError::Unsupported => connection
            .write_all(Frame::SimpleError(
                "Protocol Error: Unsupported usage of command or values".to_string(),
            ))
            .await
            .unwrap(),
    }
}
