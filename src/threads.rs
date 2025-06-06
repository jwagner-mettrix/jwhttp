use std::{
    sync::{Arc, Mutex, mpsc},
    thread
};
struct Worker {
    _id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(_id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        // will come back to this later and use std::thread::Builder::spawn()
        let thread = thread::spawn(move || {
            loop {
                let message = receiver.lock().unwrap().recv();

                match message {
                    Ok(Message::NewJob(job)) => {
                        // println!("Worker {id} got a job; executing.");
                        job();
                    }
                    Ok(Message::Terminate) => {
                        // println!("Worker {id} received terminate signal; shutting down.");
                        break;
                    }
                    Err(_) => {
                        // println!("Worker {id} disconnected; shutting down.");
                        break;
                    }
                }                
            }
        });

        Worker {
            _id,
            thread
        }
    }
}

enum Message {
    NewJob(Box<dyn FnOnce() + Send + 'static>),
    Terminate,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Message>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.as_ref().unwrap().send(Message::NewJob(job)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // println!("Sending terminate message to all workers.");

        // Send terminate message to each worker
        for _ in &self.workers {
            self.sender.as_ref().unwrap().send(Message::Terminate).unwrap();
        }

        drop(self.sender.take());

        // println!("Shutting down all workers...");

        for worker in &mut self.workers.drain(..) { 
            // println!("Shutting down worker {}", worker.id);
            worker.thread.join().unwrap();
        }
    }
}