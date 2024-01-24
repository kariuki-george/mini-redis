use mini_redis::{
    connection::{Connection, ConnectionError},
    db::DB,
    frame::{Frame, FrameError},
    runner::{Runner, RunnerError},
};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    let db = DB::new();

    loop {
        let (stream, _) = listener.accept().await.unwrap();

        let mut db = db.clone();

        let handle = tokio::spawn(async move {
            let mut connection = Connection::new(stream);

            // Read a frame to completion

            let frame = match connection.read_frame().await {
                Ok(frame) => match frame {
                    Some(frame) => frame,
                    None => Frame::Null,
                },
                Err(err) => {
                    handle_err(err, &mut connection).await;
                    return;
                }
            };

            // Parse the frame and run the commands against the db

            let mut runner = Runner::new(&mut db);
            let results = runner.run(frame);

            // Return output to the user
            match results {
                Err(err) => handle_runner_err(err, &mut connection).await,
                Ok(frame) => connection.write_all(frame).await.unwrap(),
            }

            connection.shutdown().await;
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
