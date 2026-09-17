//! A synthetic batch with nested retries and response validation, staged convergence,
//! mutable receiver state, and observable resource lifetimes. All remote
//! responses are scripted inputs; the example performs no I/O.

use std::{cell::RefCell, rc::Rc};

use kaalang::kaalang;

#[derive(Clone, Debug)]
enum Job {
    Cached(i64),
    Local(Vec<i64>),
    Remote(Vec<Attempt>),
    Skip,
    Reject(String),
}

#[derive(Clone, Debug)]
enum Attempt {
    Transient,
    Payload(Vec<String>),
    Fatal(String),
}

#[derive(Clone, Copy, Default)]
struct Policy {
    stop_after: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stop {
    #[default]
    Exhausted,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Failure {
    Exhausted,
    Fatal(String),
    Malformed { part: usize, text: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Outcome {
    Value(i128),
    Skipped,
    Rejected(String),
    Failed(Failure),
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Report {
    stop: Stop,
    outcomes: Vec<Outcome>,
    attempts: usize,
    total: i128,
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Started,
    Job(usize),
    Prepared(usize, &'static str),
    Attempt(usize, usize),
    Retry(usize),
    Parsed(usize, usize, i64),
    Validated(usize),
    Released(usize, usize),
    Processed(usize),
    Bypassed(usize, &'static str),
    Recorded(usize),
    Audited(usize),
    Finished,
}

struct Lease {
    events: Rc<RefCell<Vec<Event>>>,
    job: usize,
    attempt: usize,
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.events
            .borrow_mut()
            .push(Event::Released(self.job, self.attempt));
    }
}

#[derive(Default)]
struct BatchProcessor {
    events: Rc<RefCell<Vec<Event>>>,
    processed: usize,
    audited: usize,
    pending_audit: Vec<usize>,
}

fn sum_values(values: Vec<i64>) -> i128 {
    values.into_iter().map(i128::from).sum()
}

impl BatchProcessor {
    fn record(&mut self, report: &mut Report, outcome: Outcome, attempts: usize) {
        if let Outcome::Value(value) = &outcome {
            report.total += value;
        }
        self.events
            .borrow_mut()
            .push(Event::Recorded(report.outcomes.len()));
        report.outcomes.push(outcome);
        report.attempts += attempts;
        self.pending_audit.push(self.processed);
        self.processed += 1;
    }

    fn audit(&mut self, record: usize) {
        self.events.borrow_mut().push(Event::Audited(record));
        self.audited += 1;
    }

    #[kaalang]
    fn process_batch(&mut self, jobs: Vec<Job>, policy: Policy) -> Report {
        #[action("Open the batch.")]
        let started = |&mut self| self.events.borrow_mut().push(Event::Started);

        #[action("Prepare the queue and an empty report.")]
        let (mut pending, mut report) = |jobs| (jobs.into_iter(), Report::default());

        #[cycle("Process tasks until the queue ends or cancellation is requested.")]
        let stop = |started, &mut pending, &mut report, policy, &mut self| {
            #[question("Has the cancellation boundary been reached?")]
            #[no("Take another task.")]
            #[yes("Cancel the batch.")]
            let (poll, cancelled) = |policy, &report| {
                policy
                    .stop_after
                    .is_some_and(|limit| report.outcomes.len() >= limit)
            };

            #[action("Keep the cancellation reason.")]
            let reason = |cancelled| Stop::Cancelled;

            #[call]
            let next = |poll, &mut pending| Iterator::next(pending);

            #[choice("Is another task available?")]
            #[case("Process the task.")]
            #[case("The queue is empty.")]
            let (job, finished) = |next| match next {
                Some(job) => job,
                None => (),
            };

            #[action("Keep the normal completion reason.")]
            let reason = |finished| Stop::Exhausted;

            #[action("Record entry into this task.")]
            let index = |&job, &report, &mut self| {
                let index = report.outcomes.len();
                self.events.borrow_mut().push(Event::Job(index));
                index
            };

            #[choice("Where does the task's result come from?")]
            #[case("Use the cached value.")]
            #[case("Sum local values.")]
            #[case("Try the scripted remote responses.")]
            #[case("Skip this task.")]
            #[case("Reject this task.")]
            let (cached, local, remote, skipped, rejected) = |job| match job {
                Job::Cached(value) => value,
                Job::Local(values) => values,
                Job::Remote(responses) => responses,
                Job::Skip => (),
                Job::Reject(message) => message,
            };

            #[action("Prepare a cached value and its source.")]
            let (prepared, origin) = |cached| (i128::from(cached), "cache");

            #[call("Sum the local values without narrowing them.")]
            let local_total = |local| sum_values(local);

            #[action("Prepare a local value and its source.")]
            let (prepared, origin) = |local_total| (local_total, "local");

            #[action("Join both prepared wires before accepting the value.")]
            let (processed, attempts) = |prepared, origin, index, &mut self| {
                self.events
                    .borrow_mut()
                    .push(Event::Prepared(index, origin));
                (Outcome::Value(prepared), 0)
            };

            #[action("Prepare the remote attempt queue.")]
            let (responses, attempt_number) = |remote| (remote.into_iter(), 0usize);

            #[cycle(
                "Retry transient failures; complete on a response, a fatal error, or exhaustion."
            )]
            let (processed, attempts) = |mut responses, mut attempt_number, index, &mut self| {
                #[question("Is there another scripted response?")]
                let (try_next, exhausted) = |&responses| responses.len() > 0;

                #[action("No attempt can provide a result.")]
                let final_result = |exhausted| Outcome::Failed(Failure::Exhausted);

                #[action("Acquire a resource for this attempt.")]
                let _lease = |try_next, &mut attempt_number, index, &mut self| {
                    *attempt_number += 1;
                    self.events
                        .borrow_mut()
                        .push(Event::Attempt(index, *attempt_number));
                    Lease {
                        events: self.events.clone(),
                        job: index,
                        attempt: *attempt_number,
                    }
                };

                #[call("Read the next response.")]
                let response = |try_next, &mut responses| Iterator::next(responses);

                #[choice("Can this attempt finish the task?")]
                #[case("A transient failure can be retried.")]
                #[case("Validate the response parts.")]
                #[case("A fatal error ends the task.")]
                #[case("The final attempt failed.")]
                let (retry, payload, fatal, last_failure) = |response, &responses| match response {
                    Some(Attempt::Transient) if responses.len() > 0 => (),
                    Some(Attempt::Payload(parts)) => parts,
                    Some(Attempt::Fatal(message)) => message,
                    _ => (),
                };

                #[action("Record the retry before releasing its resource.")]
                |retry, index, &mut self| self.events.borrow_mut().push(Event::Retry(index));

                #[action("Prepare the response parts and their running total.")]
                let (parts, subtotal, part_index) = |payload| (payload.into_iter(), 0i128, 0usize);

                #[cycle("Parse each response part; discard the subtotal if any part is malformed.")]
                let checked = |mut parts, mut subtotal, mut part_index, index, &mut self| {
                    #[call]
                    let part = |&mut parts| Iterator::next(parts);

                    #[choice("Is another response part available?")]
                    #[case("Parse this part.")]
                    #[case("Every part was valid.")]
                    let (text, complete) = |part| match part {
                        Some(text) => text,
                        None => (),
                    };

                    #[action("Accept the complete response total.")]
                    let checked_result = |complete, subtotal| Outcome::Value(subtotal);

                    #[action("Borrow the part for parsing.")]
                    let source = |&text| text.as_str();

                    #[call]
                    let parsed = |source| str::parse::<i64>(source);

                    #[choice("Did the part contain an integer?")]
                    #[case("Add the parsed number.")]
                    #[case("Reject the entire response.")]
                    let (number, invalid) = |parsed| match parsed {
                        Ok(number) => number,
                        Err(_) => (),
                    };

                    #[action("Accumulate the value and record its position.")]
                    |number, &mut subtotal, &mut part_index, index, &mut self| {
                        *subtotal += i128::from(number);
                        self.events
                            .borrow_mut()
                            .push(Event::Parsed(index, *part_index, number));
                        *part_index += 1;
                    };

                    #[action("Keep the malformed part and its position.")]
                    let checked_result = |invalid, text, part_index| {
                        Outcome::Failed(Failure::Malformed {
                            part: part_index,
                            text,
                        })
                    };

                    |checked_result| break checked_result;
                };

                #[action("Finish validation while the attempt resource is still alive.")]
                let final_result = |checked, index, &mut self| {
                    self.events.borrow_mut().push(Event::Validated(index));
                    checked
                };

                #[action("Keep the fatal error without retrying.")]
                let final_result = |fatal| Outcome::Failed(Failure::Fatal(fatal));

                #[action("The final transient error exhausted the attempts.")]
                let final_result = |last_failure| Outcome::Failed(Failure::Exhausted);

                |final_result, attempt_number| break (final_result, attempt_number);
            };

            #[action("Join local and remote processing before recording the task.")]
            let (outcome, count) = |processed, attempts, index, &mut self| {
                self.events.borrow_mut().push(Event::Processed(index));
                (processed, attempts)
            };

            #[action("Prepare a skipped outcome.")]
            let (bypassed, bypass_origin) = |skipped| (Outcome::Skipped, "skip");

            #[action("Prepare a rejected outcome.")]
            let (bypassed, bypass_origin) = |rejected| (Outcome::Rejected(rejected), "reject");

            #[action("Join both bypass routes before recording the task.")]
            let (outcome, count) = |bypassed, bypass_origin, index, &mut self| {
                self.events
                    .borrow_mut()
                    .push(Event::Bypassed(index, bypass_origin));
                (bypassed, 0)
            };

            #[call("Record exactly one outcome and schedule its audit.")]
            |outcome, count, &mut report, &mut self| Self::record(self, report, outcome, count);

            |reason| break reason;
        };

        #[action("Attach the batch's completion reason.")]
        let result = |stop, mut report| {
            report.stop = stop;
            report
        };

        #[action("Take the pending audit records.")]
        let audit_queue = |&mut self| std::mem::take(&mut self.pending_audit).into_iter();

        #[cycle("Audit every recorded task before returning the report.")]
        |mut audit_queue, &mut self| {
            #[call]
            let entry = |&mut audit_queue| Iterator::next(audit_queue);

            #[choice("Is another audit record waiting?")]
            #[case("Audit the record.")]
            #[case("The audit queue is empty.")]
            let (record, flushed) = |entry| match entry {
                Some(record) => record,
                None => (),
            };

            #[call("Commit the audit record.")]
            |record, &mut self| Self::audit(self, record);

            |flushed| break;
        };

        #[action("Close the batch after all audits.")]
        |&mut self| self.events.borrow_mut().push(Event::Finished);

        |result| return result;
    }
}

#[test]
fn an_empty_batch_finishes_without_attempts_or_audits() {
    let mut processor = BatchProcessor::default();
    assert_eq!(
        processor.process_batch(vec![], Policy::default()),
        Report::default()
    );
    assert_eq!(
        *processor.events.borrow(),
        [Event::Started, Event::Finished]
    );
    assert_eq!((processor.processed, processor.audited), (0, 0));
}

fn payload(parts: &[&str]) -> Attempt {
    Attempt::Payload(parts.iter().map(|part| (*part).to_owned()).collect())
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "Keep the task routes in one test matrix."
)]
fn each_task_route_produces_one_outcome_and_releases_every_attempt() {
    let malformed = |part, text: &str| {
        Outcome::Failed(Failure::Malformed {
            part,
            text: text.to_owned(),
        })
    };
    let cases = [
        ("cache", Job::Cached(-7), Outcome::Value(-7), 0),
        ("local", Job::Local(vec![3, -2, 8]), Outcome::Value(9), 0),
        ("empty local", Job::Local(vec![]), Outcome::Value(0), 0),
        (
            "wide local sum",
            Job::Local(vec![i64::MAX, i64::MAX]),
            Outcome::Value(i128::from(i64::MAX) * 2),
            0,
        ),
        ("skip", Job::Skip, Outcome::Skipped, 0),
        (
            "reject",
            Job::Reject("blocked".into()),
            Outcome::Rejected("blocked".into()),
            0,
        ),
        (
            "no attempts",
            Job::Remote(vec![]),
            Outcome::Failed(Failure::Exhausted),
            0,
        ),
        (
            "final transient failure",
            Job::Remote(vec![Attempt::Transient]),
            Outcome::Failed(Failure::Exhausted),
            1,
        ),
        (
            "retries exhausted",
            Job::Remote(vec![
                Attempt::Transient,
                Attempt::Transient,
                Attempt::Transient,
            ]),
            Outcome::Failed(Failure::Exhausted),
            3,
        ),
        (
            "remote",
            Job::Remote(vec![payload(&["4", "-1"])]),
            Outcome::Value(3),
            1,
        ),
        (
            "empty response",
            Job::Remote(vec![payload(&[])]),
            Outcome::Value(0),
            1,
        ),
        (
            "wide remote sum",
            Job::Remote(vec![payload(&[
                "9223372036854775807",
                "9223372036854775807",
            ])]),
            Outcome::Value(i128::from(i64::MAX) * 2),
            1,
        ),
        (
            "retry then success",
            Job::Remote(vec![
                Attempt::Transient,
                payload(&["6"]),
                Attempt::Fatal("unused".into()),
            ]),
            Outcome::Value(6),
            2,
        ),
        (
            "fatal error",
            Job::Remote(vec![Attempt::Fatal("offline".into()), payload(&["99"])]),
            Outcome::Failed(Failure::Fatal("offline".into())),
            1,
        ),
        (
            "retry then fatal error",
            Job::Remote(vec![Attempt::Transient, Attempt::Fatal("offline".into())]),
            Outcome::Failed(Failure::Fatal("offline".into())),
            2,
        ),
        (
            "malformed first part",
            Job::Remote(vec![payload(&["bad", "9"]), payload(&["99"])]),
            malformed(0, "bad"),
            1,
        ),
        (
            "malformed later part discards the subtotal",
            Job::Remote(vec![payload(&["5", "bad", "9"])]),
            malformed(1, "bad"),
            1,
        ),
        (
            "out of range part",
            Job::Remote(vec![payload(&["9223372036854775808"])]),
            malformed(0, "9223372036854775808"),
            1,
        ),
    ];

    for (name, job, outcome, attempts) in cases {
        let mut processor = BatchProcessor::default();
        let total = if let Outcome::Value(value) = &outcome {
            *value
        } else {
            0
        };
        assert_eq!(
            processor.process_batch(vec![job], Policy::default()),
            Report {
                stop: Stop::Exhausted,
                outcomes: vec![outcome],
                attempts,
                total
            },
            "{name}"
        );
        assert_eq!((processor.processed, processor.audited), (1, 1), "{name}");
        assert!(processor.pending_audit.is_empty(), "{name}");
        let events = processor.events.borrow();
        let released: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                Event::Released(job, attempt) => Some((*job, *attempt)),
                _ => None,
            })
            .collect();
        assert_eq!(
            released,
            (1..=attempts)
                .map(|attempt| (0, attempt))
                .collect::<Vec<_>>(),
            "{name}"
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| **event == Event::Recorded(0))
                .count(),
            1,
            "{name}"
        );
        assert_eq!(events.last(), Some(&Event::Finished), "{name}");
    }
}

#[test]
fn a_mixed_batch_preserves_effect_order_across_all_merges_and_cycles() {
    let mut processor = BatchProcessor::default();
    let report = processor.process_batch(
        vec![
            Job::Cached(7),
            Job::Local(vec![3, -1]),
            Job::Remote(vec![
                Attempt::Transient,
                Attempt::Transient,
                payload(&["10", "-4"]),
                Attempt::Fatal("unused".into()),
            ]),
            Job::Skip,
            Job::Reject("blocked".into()),
            Job::Remote(vec![payload(&["5", "bad", "999"]), payload(&["777"])]),
        ],
        Policy::default(),
    );
    assert_eq!(
        report,
        Report {
            stop: Stop::Exhausted,
            outcomes: vec![
                Outcome::Value(7),
                Outcome::Value(2),
                Outcome::Value(6),
                Outcome::Skipped,
                Outcome::Rejected("blocked".into()),
                Outcome::Failed(Failure::Malformed {
                    part: 1,
                    text: "bad".into()
                }),
            ],
            attempts: 4,
            total: 15,
        }
    );
    assert_eq!(
        *processor.events.borrow(),
        [
            Event::Started,
            Event::Job(0),
            Event::Prepared(0, "cache"),
            Event::Processed(0),
            Event::Recorded(0),
            Event::Job(1),
            Event::Prepared(1, "local"),
            Event::Processed(1),
            Event::Recorded(1),
            Event::Job(2),
            Event::Attempt(2, 1),
            Event::Retry(2),
            Event::Released(2, 1),
            Event::Attempt(2, 2),
            Event::Retry(2),
            Event::Released(2, 2),
            Event::Attempt(2, 3),
            Event::Parsed(2, 0, 10),
            Event::Parsed(2, 1, -4),
            Event::Validated(2),
            Event::Released(2, 3),
            Event::Processed(2),
            Event::Recorded(2),
            Event::Job(3),
            Event::Bypassed(3, "skip"),
            Event::Recorded(3),
            Event::Job(4),
            Event::Bypassed(4, "reject"),
            Event::Recorded(4),
            Event::Job(5),
            Event::Attempt(5, 1),
            Event::Parsed(5, 0, 5),
            Event::Validated(5),
            Event::Released(5, 1),
            Event::Processed(5),
            Event::Recorded(5),
            Event::Audited(0),
            Event::Audited(1),
            Event::Audited(2),
            Event::Audited(3),
            Event::Audited(4),
            Event::Audited(5),
            Event::Finished,
        ]
    );
    assert_eq!((processor.processed, processor.audited), (6, 6));
    assert!(processor.pending_audit.is_empty());
}

#[test]
fn cancellation_only_happens_between_tasks_and_still_flushes_the_audit() {
    let jobs = vec![
        Job::Cached(2),
        Job::Remote(vec![Attempt::Transient, payload(&["3"])]),
        Job::Skip,
    ];
    let outcomes = [Outcome::Value(2), Outcome::Value(3), Outcome::Skipped];
    for stop_after in [Some(0), Some(1), Some(2), Some(3), Some(4), None] {
        let count = stop_after.unwrap_or(jobs.len()).min(jobs.len());
        let stop = if stop_after.is_some_and(|limit| limit <= jobs.len()) {
            Stop::Cancelled
        } else {
            Stop::Exhausted
        };
        let mut processor = BatchProcessor::default();
        assert_eq!(
            processor.process_batch(jobs.clone(), Policy { stop_after }),
            Report {
                stop,
                outcomes: outcomes[..count].to_vec(),
                attempts: if count >= 2 { 2 } else { 0 },
                total: [0, 2, 5, 5][count],
            },
            "{stop_after:?}"
        );
        assert_eq!((processor.processed, processor.audited), (count, count));
        assert!(processor.pending_audit.is_empty());
        let events = processor.events.borrow();
        let entered: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                Event::Job(index) => Some(*index),
                _ => None,
            })
            .collect();
        assert_eq!(entered, (0..count).collect::<Vec<_>>());
        let mut tail: Vec<_> = (0..count).map(Event::Audited).collect();
        tail.push(Event::Finished);
        assert!(events.ends_with(&tail));
    }
}

#[test]
fn successive_batches_keep_receiver_state_and_reset_per_batch_state() {
    let mut processor = BatchProcessor::default();
    let first = processor.process_batch(
        vec![Job::Remote(vec![Attempt::Transient, payload(&["8"])])],
        Policy::default(),
    );
    assert_eq!(first.attempts, 2);
    assert_eq!(first.total, 8);
    processor.events.borrow_mut().clear();

    assert_eq!(
        processor.process_batch(vec![Job::Local(vec![4]), Job::Skip], Policy::default()),
        Report {
            stop: Stop::Exhausted,
            outcomes: vec![Outcome::Value(4), Outcome::Skipped],
            attempts: 0,
            total: 4,
        }
    );
    assert_eq!((processor.processed, processor.audited), (3, 3));
    assert!(processor.pending_audit.is_empty());
    assert_eq!(
        *processor.events.borrow(),
        [
            Event::Started,
            Event::Job(0),
            Event::Prepared(0, "local"),
            Event::Processed(0),
            Event::Recorded(0),
            Event::Job(1),
            Event::Bypassed(1, "skip"),
            Event::Recorded(1),
            Event::Audited(1),
            Event::Audited(2),
            Event::Finished,
        ]
    );
}
