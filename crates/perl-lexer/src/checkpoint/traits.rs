use super::LexerCheckpoint;

pub trait Checkpointable {
    fn checkpoint(&self) -> LexerCheckpoint;
    fn restore(&mut self, checkpoint: &LexerCheckpoint);
    fn can_restore(&self, checkpoint: &LexerCheckpoint) -> bool;
}
