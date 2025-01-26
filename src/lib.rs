pub mod polling {

    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct PollOption {
        name: String,
        votes: usize,
    }

    impl PollOption {
        pub fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                votes: 0,
            }
        }

        pub fn inc_vote(&mut self) {
            self.votes += 1;
        }
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    #[serde(tag = "type")]
    pub enum PollSubmission {
        Radio { uuid: String, choices: String },
        Checkbox { uuid: String, choices: Vec<String> },
    }

    impl PollSubmission {
        pub fn uuid(&self) -> &String {
            match self {
                Self::Checkbox { uuid, choices } => uuid,
                Self::Radio { uuid, choices } => uuid,
            }
        }
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct ExampleForm {
        submissions: Vec<PollSubmission>,
    }

    impl ExampleForm {
        pub fn into_vec(self) -> Vec<PollSubmission> {
            self.submissions
        }
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Poll {
        uuid: String,
        title: String,
        options: Vec<PollOption>,
        multiple: bool,
    }

    pub enum PollError {
        InvalidSubmission,
    }

    type Result<T> = std::result::Result<T, PollError>;

    impl Poll {
        // need a function that finds the leader, and can report if there is a tie.

        pub fn uuid(&self) -> String {
            self.uuid.to_string()
        }

        pub fn process_submission(&mut self, submission: &PollSubmission) {
            match (submission, self.multiple) {
                (PollSubmission::Radio { uuid, choices }, false) => {
                    self.options
                        .iter_mut()
                        .filter(|option| option.name.eq(choices))
                        .for_each(|option| option.inc_vote());
                }

                (PollSubmission::Checkbox { uuid, choices }, true) => {
                    self.options
                        .iter_mut()
                        .filter(|option| choices.contains(&option.name))
                        .for_each(|option| {
                            option.inc_vote();
                        });
                }
                _ => {}
            }
        }
        pub fn multiple_choice(title: &str) -> Self {
            let uuid = Uuid::new_v4();
            Self {
                uuid: uuid.to_string(),
                title: title.to_string(),
                options: vec![],
                multiple: true,
            }
        }

        pub fn new(title: &str) -> Self {
            let mut item = Self::multiple_choice(title);
            item.multiple = false;
            item
        }
        pub fn add_option(&mut self, name: &str) {
            self.options.push(PollOption::new(name));
        }
    }

    #[derive(Serialize, Deserialize, Clone, Default)]
    pub struct PollCollection {
        polls: Vec<Poll>,
    }

    impl PollCollection {
        pub fn send_submission(&mut self, submission: PollSubmission) {
            self.polls
                .iter_mut()
                .filter(|poll| poll.uuid().eq(submission.uuid()))
                .for_each(|poll| poll.process_submission(&submission));
        }
        pub fn push_poll(&mut self, poll: Poll) {
            self.polls.push(poll);
        }

        pub fn get_mut(&mut self, name: &str) -> Option<&mut Poll> {
            self.polls.iter_mut().find(|poll| poll.title.eq(name))
        }
    }
}

pub mod app {

    // TODO nest routes into app and make it a config for actix-web.
    use crate::polling::{Poll, PollCollection};
    use serde::{Deserialize, Serialize};
    use std::{fs::OpenOptions, path::PathBuf};

    #[derive(Serialize, Deserialize)]
    pub struct Config {
        dates: Vec<String>,
        movies: Option<Vec<String>>,
    }

    impl TryFrom<PathBuf> for Config {
        type Error = std::io::Error;
        fn try_from(value: PathBuf) -> std::io::Result<Self> {
            let file = OpenOptions::new().read(true).open(value)?;
            Ok(serde_json::from_reader(file)?)
        }
    }
    impl Config {
        pub fn make_polls(self) -> PollCollection {
            // todo need to add movies here later maybe.
            let mut collection = PollCollection::default();
            let mut dates_poll = Poll::multiple_choice("Dates");

            //let dates_poll = collection.new_poll("Dates", true);
            let mut movies_poll = Poll::new("Movies");

            self.dates.iter().for_each(|date| {
                dates_poll.add_option(date);
            });

            if let Some(movies) = self.movies {
                movies.iter().for_each(|movie| {
                    movies_poll.add_option(movie);
                });
            }

            collection.push_poll(dates_poll);
            collection.push_poll(movies_poll);

            collection
        }
    }
}

pub mod routes {

    use std::sync::Mutex;

    use crate::polling::{ExampleForm, PollCollection};
    use actix_web::{
        get, post,
        web::{Data, Json},
        HttpResponse,
    };

    #[get("/health_check")]
    async fn health_check() -> HttpResponse {
        HttpResponse::Ok().finish()
    }

    #[post("/submit.new")]
    async fn submit_new_form(
        form: Json<ExampleForm>,
        polls: Data<Mutex<PollCollection>>,
    ) -> HttpResponse {
        if let Ok(mut polls) = polls.lock() {
            form.0.into_vec().into_iter().for_each(|submission| {
                polls.send_submission(submission);
            });
        }
        HttpResponse::Ok().finish()
    }

    #[get("/polls")]
    async fn get_poll(poll: Data<Mutex<PollCollection>>) -> HttpResponse {
        println!("sending polling data");
        HttpResponse::Ok().json(poll)
    }
}
