use serde::{Deserialize, Serialize};
use serde_json;

pub mod polling {
    use std::collections::HashMap;

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
    pub enum PollSubmission {
        Single(String),
        Multiple(Vec<String>),
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    #[serde(tag = "type")]
    pub enum NewPollSubmission {
        Radio { uuid: String, choices: String },
        Checkbox { uuid: String, choices: Vec<String> },
    }

    impl NewPollSubmission {
        pub fn uuid(&self) -> &String {
            match self {
                Self::Checkbox { uuid, choices } => uuid,
                Self::Radio { uuid, choices } => uuid,
            }
        }
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct ExampleForm {
        submissions: Vec<NewPollSubmission>,
    }

    impl ExampleForm {
        pub fn into_vec(self) -> Vec<NewPollSubmission> {
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
        pub fn add_submission(&mut self, submission: PollSubmission) -> Result<()> {
            match (self.multiple, submission) {
                (false, PollSubmission::Single(item)) => {
                    self.options
                        .iter_mut()
                        .filter(|option| option.name == item)
                        .for_each(|option| option.inc_vote());
                    Ok(())
                }
                (true, PollSubmission::Multiple(items)) => {
                    self.options
                        .iter_mut()
                        .filter(|option| items.contains(&option.name))
                        .for_each(|option| option.inc_vote());
                    Ok(())
                }
                _ => Err(PollError::InvalidSubmission),
            }
        }

        pub fn uuid(&self) -> String {
            self.uuid.to_string()
        }

        pub fn process_submission(&mut self, submission: &NewPollSubmission) {
            match (submission, self.multiple) {
                (NewPollSubmission::Radio { uuid, choices }, false) => {
                    self.options
                        .iter_mut()
                        .filter(|option| option.name.eq(choices))
                        .for_each(|option| option.inc_vote());
                }

                (NewPollSubmission::Checkbox { uuid, choices }, true) => {
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
        pub fn send_submission(&mut self, submission: NewPollSubmission) {
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
    use std::{fs::OpenOptions, path::PathBuf};

    use serde::{Deserialize, Serialize};

    use crate::polling::{Poll, PollCollection, PollSubmission};

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
    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct FormSubmission {
        dates: Vec<String>,
        movie: Option<String>,
    }

    impl Into<Vec<PollSubmission>> for FormSubmission {
        fn into(self) -> Vec<PollSubmission> {
            let submission = PollSubmission::Multiple(self.dates);
            let mut subs = vec![submission];

            if let Some(movie) = self.movie {
                subs.push(PollSubmission::Single(movie));
            }
            subs
        }
    }
    pub mod testing {
        use super::Config;
        pub fn example_config() -> Config {
            Config {
                dates: vec!["Feb 15th, 2025".to_string(), "Feb 28th, 2025".to_string()],
                movies: None,
            }
        }
    }
}

pub mod routes {

    use std::{fs::OpenOptions, sync::Mutex};

    use crate::{
        app::FormSubmission,
        polling::{ExampleForm, PollCollection},
    };

    use super::polling::{Poll, PollSubmission};
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
        println!("{:?}", &form);
        if let Ok(mut polls) = polls.lock() {
            form.0.into_vec().into_iter().for_each(|submission| {
                polls.send_submission(submission);
            });
        }
        HttpResponse::Ok().finish()
    }

    #[post("/submit")]
    async fn form_submit(
        form: Json<FormSubmission>,
        polls: Data<Mutex<PollCollection>>,
    ) -> HttpResponse {
        let submissions: Vec<PollSubmission> = form.0.into();
        println!("received a submission -> {:?}", submissions);

        // Mappings here are dummy stupid now.
        // Data expected on the return should be closer tied
        // to what is actually being returned.

        // The front-end is so far responsive enough to just take anything.

        if let Ok(mut polls) = polls.lock() {
            let res = polls
                .get_mut("Dates")
                .unwrap()
                .add_submission(submissions[0].clone());
            if !res.is_ok() {
                println!("error on submission. Could be in an invalid format?")
            }

            let res = polls
                .get_mut("Movies")
                .unwrap()
                .add_submission(submissions[1].clone());
            if !res.is_ok() {
                println!("error on submission. Could be in an invalid format?")
            }
        }

        HttpResponse::Ok().finish()
    }

    #[get("/polls")]
    async fn get_poll(poll: Data<Mutex<PollCollection>>) -> HttpResponse {
        println!("sending polling data");
        HttpResponse::Ok().json(poll)
    }
}
