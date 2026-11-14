use rand::{Rng, RngExt};
use serde::Deserialize;
use serde_json::{Deserializer, Result, Value};
use std::{
    collections::HashMap,
    fs,
    io::Read,
    time::{Duration, SystemTime},
};
use strum::EnumIter;
#[derive(Default, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub answer_letter: String,
    pub answer_text: String,
    pub correct: bool,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub question_num: usize,
    pub question_text: String,
    pub answers: Vec<Answer>,
    #[serde(skip)]
    pub user_answer: Option<Answer>,
}

pub type QuestionBank = Vec<Question>;
#[derive(Clone, Debug)]
pub struct Exam {
    pub title: String,
    raw_data: Option<String>,
    pub question_bank: Option<QuestionBank>,
    pub exam_questions: Option<QuestionBank>,
    pub max_questions: usize,
    pub time_elapsed: Duration,
    pub time_started: SystemTime,
    pub max_time: Option<Duration>,
    pub total_questions: usize,
    pub total_bank_questions: usize,
    pub source_path: Option<String>,
    pub current_question: Option<usize>,
    pub did_load: bool,
    pub did_generate: bool,
}

impl Exam {
    pub fn new() -> Exam {
        Exam {
            title: "Multiple Choice Exam".to_string(),
            question_bank: None,
            exam_questions: None,
            max_questions: 100,
            total_questions: 0,
            time_elapsed: Duration::ZERO,
            time_started: SystemTime::now(),
            max_time: None,
            raw_data: None,
            total_bank_questions: 0,
            source_path: None,
            current_question: None,
            did_load: false,
            did_generate: false,
        }
    }

    pub fn set_max_questions(&mut self, new_string: &str) -> &mut Self {
        let res = new_string.parse::<usize>();
        match res {
            Ok(num) => self.max_questions = num,
            Err(err) => {}
        }

        self
    }

    pub fn generate_exam(&mut self) -> &mut Self {
        self.exam_questions = None;
        let mut rng = rand::rng();
        let mut _exam_questions = QuestionBank::new();
        if let Some(_questionbank) = &self.question_bank {
            let mut questionbank = _questionbank.clone();
            let mut maxq: usize = 1;
            let mut max_total: usize = self.total_bank_questions;
            while maxq < self.max_questions {
                let rand_index = rng.random_range(0..max_total);
                let mut pulled_question = questionbank.swap_remove(rand_index);
                pulled_question.question_num = maxq;
                _exam_questions.push(pulled_question);

                max_total -= 1;
                maxq += 1;
            }
        }
        self.exam_questions = Some(_exam_questions);
        self.current_question = Some(1);
        self
    }
    pub fn load_file(&mut self, file_path: &str) -> &mut Self {
        let contents = fs::read_to_string(file_path).ok();
        self.source_path = Some(file_path.to_string());
        self.raw_data = contents;
        // if let Some(debug_data) = &self.raw_data {
        //     println!("debugData {debug_data}");
        // }
        self
    }
    pub fn load_questions(&mut self) -> &mut Self {
        if let Some(data) = &self.raw_data {
            let questions: Vec<Question> = serde_json::from_str(data).unwrap();
            // println!("{:?}", questions);
            let mut _question_bank = QuestionBank::new();
            let mut count = 0;

            for question in questions {
                _question_bank.push(question);
                count += 1;
            }
            self.question_bank = Some(_question_bank);
            self.total_bank_questions = count;
        }

        self
    }
}
