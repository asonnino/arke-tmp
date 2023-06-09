use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use config::Committee;
use messages::{Certificate, ClientToAuthorityMessage, Digest, Vote, WriteTransaction};
use rand::{rngs::StdRng, SeedableRng};
use tokio::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        Notify,
    },
    task::JoinHandle,
};

/// Epoch number of all transactions in the benchmark.
pub const BENCHMARK_EPOCH: u64 = 1;

/// Make dumb (but valid) write transactions.
pub struct WriteTransactionGenerator {
    notify: Arc<Notify>,
    _tx_transaction: Sender<(Digest, Bytes)>,
    rx_transaction: Receiver<(Digest, Bytes)>,
    _handler: JoinHandle<()>,
}

impl WriteTransactionGenerator {
    pub fn new(size: usize, pre_generation: usize) -> Self {
        let (tx_transaction, rx_transaction) = channel(pre_generation * 10);
        let notify = Arc::new(Notify::new());
        let cloned_notify = notify.clone();

        // Generate new transactions in the background.
        let sender = tx_transaction.clone();
        let handler = tokio::spawn(async move {
            let mut csprng = StdRng::from_entropy();

            let mut i = 0;
            loop {
                if i % pre_generation == 0 {
                    tracing::debug!("Generated {i} tx");
                }

                let version = 1;
                let epoch = BENCHMARK_EPOCH;
                let tx =
                    WriteTransaction::rand::<StdRng>(Some(version), Some(epoch), size, &mut csprng)
                        .unwrap();
                let id = tx.id.clone();

                let message = ClientToAuthorityMessage::WriteTransaction(tx);
                let serialized = bincode::serialize(&message).unwrap();
                let bytes = Bytes::from(serialized);

                i += 1;
                if i == pre_generation {
                    cloned_notify.notify_one();
                }

                // This call blocks when the channel is full.
                sender.send((id, bytes)).await.expect("Failed to send tx");
                tokio::task::yield_now().await;
            }
        });

        Self {
            notify,
            _tx_transaction: tx_transaction,
            rx_transaction,
            _handler: handler,
        }
    }

    pub async fn make_tx(&mut self) -> (Digest, Bytes) {
        self.rx_transaction
            .recv()
            .await
            .expect("Failed to get new tx")
    }

    pub async fn initialize(&self) {
        self.notify.notified().await;
    }
}

/// Make dumb (but valid) publish certificates.
pub struct CertificateAggregator {
    /// The committee information.
    pub committee: Committee,
    /// The number of correct authorities.
    num_of_correct_authorities: usize,
    /// Buffer holding the votes received so far.
    map: HashMap<Digest, Vec<Vote>>,
}

impl CertificateAggregator {
    pub fn new(committee: Committee, num_of_correct_authorities: usize) -> Self {
        Self {
            committee,
            map: HashMap::with_capacity(10_000),
            num_of_correct_authorities,
        }
    }

    /// Reset the certificate generator.
    pub fn clear(&mut self) {
        self.map
            .retain(|_k, v| v.len() != self.num_of_correct_authorities)
    }

    /// Try to assemble a certificate from votes.
    pub fn try_make_certificate(&mut self, vote: Vote) -> Option<(Digest, Bytes)> {
        let tx = vote.transaction.clone();
        let id = tx.id.clone();
        let votes = self.map.entry(id).or_insert_with(Vec::new);
        votes.push(vote);

        (votes.len() >= self.committee.quorum_threshold() as usize).then(|| {
            let certificate: Certificate = (tx, votes.drain(..)).into();
            let id = certificate.id().clone();

            let message = ClientToAuthorityMessage::Certificate(certificate);
            let serialized = bincode::serialize(&message).unwrap();
            (id, Bytes::from(serialized))
        })
    }
}

/// Make dumb (but valid) publish certificates.
pub struct AckAggregator {
    /// The committee information.
    pub committee: Committee,
    /// The number of correct authorities.
    num_of_correct_authorities: usize,
    /// Buffer holding the number of certificates acknowledgements received.
    map: HashMap<Digest, usize>,
}

impl AckAggregator {
    pub fn new(committee: Committee, num_of_correct_authorities: usize) -> Self {
        Self {
            committee,
            map: HashMap::with_capacity(10_000),
            num_of_correct_authorities,
        }
    }

    /// Reset the certificate generator.
    pub fn clear(&mut self) {
        self.map
            .retain(|_k, v| *v != self.num_of_correct_authorities)
    }

    /// Try to assemble a certificate from votes.
    pub fn check_ack_quorum(&mut self, id: Digest) -> bool {
        let acks = self.map.entry(id).or_insert_with(|| 0);
        *acks += 1;
        if *acks >= self.committee.quorum_threshold() as usize {
            *acks = 0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod test {
    use super::WriteTransactionGenerator;

    #[tokio::test]
    async fn pre_generate_transactions() {
        let mut generator = WriteTransactionGenerator::new(32, 10);
        generator.initialize().await;
        for _ in 0..10 {
            let _ = generator.make_tx().await;
        }
    }
}
