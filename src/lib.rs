use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        // Make sure the size is greater than 0, otherwise the pool
        // would contain no threads.
        assert!(size > 0);

        // Create a channel to communicate with the threads in the pool.
        // The channel is created with `mpsc::channel()`, which returns a
        // tuple containing a sender and a receiver. We'll use the sender to
        // send jobs to the threads in the pool.
        let (sender, receiver) = mpsc::channel();

        // We'll use the receiver to receive jobs from the sender. We need to
        // wrap the receiver in an `Arc` and a `Mutex` so that it can be shared
        // among multiple threads.
        let receiver = Arc::new(Mutex::new(receiver));

        // Create a vector to store the workers. We'll use the `with_capacity()`
        // function to pre-allocate space for the vector, so that we don't have
        // to reallocate it every time we add a worker.
        let mut workers = Vec::with_capacity(size);

        // Create a worker for each thread in the pool. We'll use the `clone()`
        // method to create a clone of the receiver for each worker, so that
        // each worker has its own copy of the receiver.
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        // Create a ThreadPool instance with the workers and the sender.
        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    /// Execute a closure on a thread in the pool.
    ///
    /// The `execute()` method takes a closure as an argument, and sends it
    /// to one of the threads in the pool. The closure is executed on the
    /// thread, and the result is discarded.
    ///
    /// The `execute()` method is safe to call from any thread, as it uses a
    /// channel to communicate with the threads in the pool. The channel is
    /// created with `mpsc::channel()`, which creates a sender and a receiver.
    /// The sender is used to send the job to the threads in the pool, and the
    /// receiver is used to receive the result of the job.
    ///
    /// The `execute()` method will panic if the sender is not available,
    /// which means that the pool has been shut down. This is a bug, as the
    /// pool should not be shut down until all the jobs have been completed.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // Create a Box around the closure. This is necessary because the
        // closure is a trait object, and trait objects can't be sent over a
        // channel.
        let job = Box::new(f);

        // Get a reference to the sender. The sender is stored in the
        // `ThreadPool` instance, and it's used to send jobs to the threads in
        // the pool.
        let sender = self.sender.as_ref().unwrap();

        // Send the job to the threads in the pool. The `send()` method takes a
        // `Job` as an argument, and sends it to one of the threads in the pool.
        // The `send()` method will block until the job is sent, so it's safe to
        // call from any thread.
        sender.send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    /// When the `ThreadPool` is dropped, we need to shut down all the threads
    /// in the pool. This is done by taking the sender, which will cause all
    /// the threads in the pool to exit when they try to receive a job.
    ///
    /// We then iterate over the workers in the pool, and for each one, we
    /// print a message saying that we're shutting down the worker. We then
    /// take the thread from the worker, and call `join()` on it. This will
    /// block until the thread has finished, and then we can drop the thread.
    ///
    /// Note that we don't need to explicitly drop the workers, as they will
    /// be dropped when the `ThreadPool` is dropped.
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        // Spawn a new thread for the worker. The `move` keyword ensures that
        // the closure takes ownership of the `id` and `receiver` variables.
        let thread = thread::spawn(move || loop {
            // Attempt to receive a job from the channel. The `lock().unwrap()`
            // call locks the `Mutex` and panics if the lock is poisoned. The
            // `recv()` method returns a `Result` which is `Ok(job)` if a job
            // was received or `Err(_)` if the channel is disconnected.
            let message = receiver.lock().unwrap().recv();

            match message {
                // If a job was received, print a message and execute the job.
                Ok(job) => {
                    println!("Worker {id} got a job; executing.");

                    // Call the job, which is a closure.
                    job();
                }
                // If the channel is disconnected, print a message and break
                // the loop to terminate the thread.
                Err(_) => {
                    println!("Worker {id} disconnected; shutting down.");
                    break;
                }
            }
        });

        // Return a new `Worker` instance with the given `id` and the spawned
        // thread wrapped in `Some`.
        Worker {
            id,
            thread: Some(thread),
        }
    }
}