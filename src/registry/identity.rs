//! Identity registry: `did:bud`, credential commitments, guardian recovery.
//!
//! The architecture this module lands is the registry-layer answer of the
//! identity design doc: no consensus mechanism is invented, no raw identity
//! data is ever placed on chain. What this type holds are *commitments* -
//! hash roots over fields the holder proved knowledge of - and the small
//! rules that make a commitment worth reading: who may write, when a key is
//! live, what a revocation costs, and which quorum can override a lost key.
//!
//! # The two halves
//!
//! [`IdentityRecord`] is the DID side: `did:bud:<address>` maps to the
//! address itself, and the record carries the verification methods (key
//! handles, [MethodKind::MlDsa87] today, because `ml-dsa-87` is the node's
//! own signature primitive), the current credential root, and the guardian
//! set with its recovery threshold. [`CredentialCommitment`] is the
//! Verifiable-Credentials side: a closed field list where every field is a
//! [`FieldCommitment`] (a hash over schema, name, salt and value digest -
//! never the value), and the root is a plain Merkle fold over the fields, so
//! selective disclosure is a sibling path, not a new proof system.
//!
//! # Domain placement
//!
//! Write authority lives on the PoA domain; the gate is [`ConsensusKind::PoA`]
//! and it is enforced by the registry itself, not left to callers, because a
//! rule a caller can forget is a rule that will be forgotten. Other domains
//! verify against an anchored root - that part is the cross-domain anchor
//! slice and is deliberately absent here: this module is pure state, it has
//! no block access, and pretending otherwise would wire the anchor into the
//! wrong door.
//!
//! # What "verified" can honestly mean off-chain
//!
//! [`IdentityRegistry::is_credential_valid`] answers "was this credential
//! issued to a registered subject, unrevoked, and unexpired at `now`, with a
//! root that recomputes from its own fields?" It does not and cannot answer
//! "is the data true" - the chain holds commitments to claims, the claims
//! live with their holders, and an issuer's signature is verified where the
//! signature is made (the grant-side precedent:
//! [`crate::storage::view_grant::GrantAuthorization::verify`]).
//!
//! # Disclosure binding
//!
//! A disclosure proves one field against a root. It does not say who is
//! reading. The consent flow this system is designed for - the wallet screen
//! that shows the requester which field will open, and the rule that the
//! opened value lands only in that requester's wallet - lives in the
//! view-grant layer, which already binds grants to grantees and revokes by
//! digest. Identity supplies the field commitments that make "which one
//! field" a checkable question rather than a sentence in a consent dialog.

use crate::core::address::Address;
use crate::core::hash::hash_fields_bytes;
use crate::domain::ConsensusKind;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A [`ConsensusKind::Custom`] label that must be written exactly this way
/// before a PoA-only write is refused: the gate matches on the constant, not
/// on a string an operator can mistype into an open door.
pub const DID_METHOD_NAME: &str = "did:bud";

/// Formats the DID for an address. Lowercase hex over the 32 address bytes,
/// so a DID survives case-folding in any wallet that stores it as text.
#[must_use]
pub fn did_of(address: &Address) -> String {
    let mut out = String::with_capacity(DID_METHOD_NAME.len() + 1 + 64);
    out.push_str(DID_METHOD_NAME);
    out.push(':');
    for byte in address.as_bytes() {
        out.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
        out.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap_or('0'));
    }
    out
}

///
/// WIRING: not yet called in production; the RPC resolver that reads DIDs
/// off the wire is the consumer this is written for.
/// Parses a `did:bud:<64 hex>` string back to an address. Anything else -
/// wrong method, odd length, non-hex - is `None`, silently, because the
/// parse has no opinion to report; callers that must distinguish refusals
/// use [`CredentialFieldRule`] errors on the registry doors instead.
#[must_use]
pub fn address_of_did(did: &str) -> Option<Address> {
    let hex = did.strip_prefix(DID_METHOD_NAME)?.strip_prefix(':')?;
    let bytes = hex.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let mut raw = [0u8; 32];
    // chunks_exact + zip: `bytes.len() == 64` is checked above, so the
    // chunker yields exactly 32 pairs and the fold has no index to get
    // out of range - the old `bytes[i * 2]` relied on that arithmetic
    // being right; this relies on the iterator's type instead.
    for (pair, out) in bytes.chunks_exact(2).zip(raw.iter_mut()) {
        let hi = hex_val(pair[0])?;
        let lo = hex_val(pair[1])?;
        *out = (hi << 4) | lo;
    }
    // Lowercase-only rule: an "uppercase DID" is the same key two spellings
    // away from a different registry entry, which is exactly how a DID gets
    // duplicated by copy-paste. Refuse it here rather than normalize it later.
    if did_of(&Address(raw)) == did {
        Some(Address(raw))
    } else {
        None
    }
}

/// Lowercase hex for registry keys (see [`IdentityRegistry::credentials`]
/// for why ids are stored hexed rather than as byte arrays).
fn hex32(bytes: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
        out.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap_or('0'));
    }
    out
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

/// Which signature scheme a key handle speaks. Closed on purpose: a registry
/// that accepts "some key" is a registry that accepts nothing verifiable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MethodKind {
    /// The node's post-quantum default; verified by
    /// `crate::crypto::primitives::verify_ml_dsa_87_signature` at the doors
    /// that consume signatures (grant precedent).
    MlDsa87,
}

/// One verification method of a DID: a 32-byte key handle, the scheme it
/// speaks, and the epoch it was revoked at, if it was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub key_id: [u8; 32],
    pub kind: MethodKind,
    pub revoked_at: Option<u64>,
}

impl VerificationMethod {
    #[must_use]
    pub fn new(key_id: [u8; 32], kind: MethodKind) -> Self {
        Self {
            key_id,
            kind,
            revoked_at: None,
        }
    }

    /// Live at `now`: a revocation takes effect at its own epoch, not after
    /// it - a key revoked at 10 is not live at 10.
    #[must_use]
    pub fn is_live_at(&self, now: u64) -> bool {
        self.revoked_at.is_none_or(|epoch| now < epoch)
    }
}

/// The DID side of the registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityRecord {
    pub subject: Address,
    pub methods: Vec<VerificationMethod>,
    /// The credential root currently asserted for this subject: the Merkle
    /// root of the credential the subject last issued itself. Absent until
    /// the first credential is issued - there is no "empty root" state a
    /// reader could mistake for "issued, then revoked".
    pub credential_root: Option<[u8; 32]>,
    pub guardians: Vec<Address>,
    pub recovery_threshold: usize,
}

impl IdentityRecord {
    ///
    /// WIRING: production entry arrives with the identity transaction door.
    /// Builds and validates a record in one step: a record that fails
    /// [`IdentityRecord::validate`] is not constructible from public types
    /// without going through the registry, which runs this first.
    ///
    /// # Errors
    ///
    /// The first [`IdentityError`] found.
    pub fn new(
        subject: Address,
        methods: Vec<VerificationMethod>,
        guardians: Vec<Address>,
        recovery_threshold: usize,
    ) -> Result<Self, IdentityError> {
        let record = Self {
            subject,
            methods,
            credential_root: None,
            guardians,
            recovery_threshold,
        };
        record.validate()?;
        Ok(record)
    }

    /// # Errors
    ///
    /// First structural violation found.
    pub fn validate(&self) -> Result<(), IdentityError> {
        if self.methods.is_empty() {
            return Err(IdentityError::NoMethods {
                did: did_of(&self.subject),
            });
        }
        for (i, method) in self.methods.iter().enumerate() {
            if self
                .methods
                .iter()
                .take(i)
                .any(|m| m.key_id == method.key_id)
            {
                return Err(IdentityError::DuplicateKeyId {
                    did: did_of(&self.subject),
                });
            }
        }
        for (i, guardian) in self.guardians.iter().enumerate() {
            if guardian == &self.subject {
                return Err(IdentityError::SubjectIsOwnGuardian {
                    did: did_of(&self.subject),
                });
            }
            if self.guardians.iter().take(i).any(|g| g == guardian) {
                return Err(IdentityError::DuplicateGuardian {
                    did: did_of(&self.subject),
                });
            }
        }
        if self.guardians.is_empty() {
            if self.recovery_threshold != 0 {
                return Err(IdentityError::ThresholdWithoutGuardians {
                    did: did_of(&self.subject),
                });
            }
        } else if self.recovery_threshold == 0 || self.recovery_threshold > self.guardians.len() {
            return Err(IdentityError::UnreachableQuorum {
                did: did_of(&self.subject),
                threshold: self.recovery_threshold,
                guardians: self.guardians.len(),
            });
        }
        Ok(())
    }

    ///
    /// WIRING: production entry arrives with the signature-checking doors.
    /// A method live at `now`; the lookup is by key handle, the same
    /// `[u8; 32]` a signature envelope carries.
    #[must_use]
    pub fn live_method(&self, key_id: &[u8; 32], now: u64) -> Option<&VerificationMethod> {
        self.methods
            .iter()
            .find(|m| &m.key_id == key_id && m.is_live_at(now))
    }
}

/// A committed field: the only unit a credential discloses. The name is
/// public (the consent screen shows exactly these), the value is not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldCommitment {
    pub name: String,
    pub commitment: [u8; 32],
}

/// The commitment a field carries: `H(domain | schema | name | salt | value digest)`.
///
/// The salt is what makes this a commitment rather than a hash of a
/// low-entropy fact: a birth date hashed alone is a guessable dictionary,
/// and the chain must not be a place where "did this subject have
/// credential X" is answerable by trying every date. The value enters as a
/// digest so the preimage of the value itself never has to.
#[must_use]
pub fn field_commitment(
    schema: &str,
    name: &str,
    salt: &[u8; 32],
    value_digest: &[u8; 32],
) -> [u8; 32] {
    hash_fields_bytes(&[
        b"bud-vc-v1-field",
        schema.as_bytes(),
        name.as_bytes(),
        salt,
        value_digest,
    ])
}

/// Node-pair hash of the disclosure tree. Order matters and is baked into
/// the domain tag: the same fields in another order are another root, which
/// is what lets the read-back check refuse a reordered credential.
#[must_use]
pub fn field_pair_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    hash_fields_bytes(&[b"bud-vc-v1-node", left, right])
}

/// The Verifiable-Credential side: issuer, subject, schema, and the closed
/// field list. Values are not here; they cannot be, by construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialCommitment {
    pub issuer: Address,
    pub subject: Address,
    pub schema: String,
    pub fields: Vec<FieldCommitment>,
    pub issued_at: u64,
    pub expires_at: Option<u64>,
}

impl CredentialCommitment {
    /// # Errors
    ///
    /// [`IdentityError::BadSchema`] for an empty/oversized schema label or a
    /// field name that is empty; [`IdentityError::NoFields`] for a credential
    /// committing to nothing (a root over an empty list is a constant, and a
    /// constant root means every such credential "matches" every other).
    pub fn validate(&self) -> Result<(), IdentityError> {
        if self.schema.is_empty() || self.schema.len() > 64 || self.fields.is_empty() {
            if self.fields.is_empty() {
                return Err(IdentityError::NoFields);
            }
            return Err(IdentityError::BadSchema {
                schema: self.schema.clone(),
            });
        }
        for (i, field) in self.fields.iter().enumerate() {
            if field.name.is_empty() || field.name.len() > 64 {
                return Err(IdentityError::BadSchema {
                    schema: format!("field name `{}`", field.name),
                });
            }
            if self.fields.iter().take(i).any(|f| f.name == field.name) {
                return Err(IdentityError::DuplicateField {
                    name: field.name.clone(),
                });
            }
        }
        Ok(())
    }

    /// The Merkle fold over the field commitments in order; an odd node at
    /// any level is paired with itself (the chain's own convention for
    /// binary folds: no leaf gets a special "I am alone" hash, so the proof
    /// walk needs no exceptions).
    #[must_use]
    pub fn root(&self) -> [u8; 32] {
        merkle_root(&self.field_leaves())
    }

    /// The leaves in document order - the exact sequence the disclosure
    /// tree is folded from, public so a wallet and a node build paths from
    /// the same list without conversing.
    #[must_use]
    pub fn field_leaves(&self) -> Vec<[u8; 32]> {
        self.fields.iter().map(|f| f.commitment).collect()
    }

    #[must_use]
    pub fn is_live_at(&self, now: u64) -> bool {
        self.issued_at <= now && self.expires_at.is_none_or(|exp| now < exp)
    }
}

/// A disclosure: one field's sibling path, positional. The positions come
/// with the proof, and the verifier re-walks them; there is no "trust me,
/// it was leaf 3" anywhere in the format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisclosureProof {
    pub leaf_index: usize,
    pub leaf_count: usize,
    pub siblings: Vec<[u8; 32]>,
}

/// Builds the proof for `leaf_index`.
#[must_use]
pub fn disclosure_proof(leaves: &[[u8; 32]], leaf_index: usize) -> Option<DisclosureProof> {
    if leaves.is_empty() || leaf_index >= leaves.len() {
        return None;
    }
    let mut level: Vec<[u8; 32]> = leaves.to_vec();
    let mut index = leaf_index;
    let mut siblings = Vec::new();
    while level.len() > 1 {
        let pair = if index % 2 == 0 { index + 1 } else { index - 1 };
        // `pair` may run past the level (odd tail); the leaf then pairs
        // with itself - fetched by `get` so no index expression exists.
        let sib = *level
            .get(pair)
            .or_else(|| level.get(index))
            .unwrap_or(&[0u8; 32]);
        siblings.push(sib);
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for chunk in level.chunks(2) {
            let left = chunk[0];
            let right = chunk.get(1).copied().unwrap_or(left);
            next.push(field_pair_hash(&left, &right));
        }
        level = next;
        index /= 2;
    }
    Some(DisclosureProof {
        leaf_index,
        leaf_count: leaves.len(),
        siblings,
    })
}

/// Recomputes the leaf from the disclosure material and walks it to a root.
/// True only if that root is the expected one; every failure mode (wrong
/// salt, wrong value digest, wrong name, tampered path) lands in `false`,
/// never in a panic or a "looks close enough".
#[must_use]
pub fn verify_disclosure(
    root: &[u8; 32],
    schema: &str,
    name: &str,
    salt: &[u8; 32],
    value_digest: &[u8; 32],
    proof: &DisclosureProof,
) -> bool {
    let mut level = proof.leaf_count;
    let mut index = proof.leaf_index;
    if level == 0 || index >= level || proof.siblings.len() != sibling_count(level) {
        return false;
    }
    let mut current = field_commitment(schema, name, salt, value_digest);
    for sibling in &proof.siblings {
        current = if index % 2 == 0 {
            field_pair_hash(&current, sibling)
        } else {
            field_pair_hash(sibling, &current)
        };
        level = level.div_ceil(2);
        index /= 2;
    }
    &current == root
}

fn sibling_count(leaves: usize) -> usize {
    let mut count = 0;
    let mut level = leaves;
    while level > 1 {
        count += 1;
        level = level.div_ceil(2);
    }
    count
}

#[must_use]
pub fn merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    let mut level: Vec<[u8; 32]> = leaves.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for chunk in level.chunks(2) {
            let left = chunk[0];
            let right = chunk.get(1).copied().unwrap_or(left);
            next.push(field_pair_hash(&left, &right));
        }
        level = next;
    }
    level[0]
}

/// The registry. Pure state: it never touches a clock or a block. Every
/// method takes `now` and a `domain`, and the refusal rules are here rather
/// than at the doors, so any future caller inherits them by construction.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityRegistry {
    records: BTreeMap<Address, IdentityRecord>,
    /// Credential id = H(root | issuer | issued_at): two credentials with
    /// the same fields at different times are different credentials, and
    /// neither can shadow the other's revocation.
    /// Hex-keyed on purpose: a `[u8; 32]` map key serializes as an array,
    /// and the snapshot is JSON - a populated registry would fail at
    /// write time, not at compile time. The id bytes stay the public handle;
    /// the encoding is this type's business. The same reasoning is the
    /// `Address` newtype's manual `Serialize`.
    credentials: BTreeMap<String, CredentialCommitment>,
    revoked: BTreeSet<String>,
}

impl IdentityRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn record(&self, subject: &Address) -> Option<&IdentityRecord> {
        self.records.get(subject)
    }

    #[must_use]
    pub fn credential(&self, id: &[u8; 32]) -> Option<&CredentialCommitment> {
        self.credentials.get(hex32(id).as_str())
    }

    #[must_use]
    pub fn is_revoked(&self, id: &[u8; 32]) -> bool {
        self.revoked.contains(hex32(id).as_str())
    }

    /// The only door every mutation passes. A non-PoA `domain` refuses
    /// anything, before any state is read or validated: the write-authority
    /// rule does not leak even the shape of "would have been accepted".
    ///
    /// # Errors
    ///
    /// [`IdentityError::NotPoaDomain`] plus whatever the operation itself
    /// fails on.
    pub fn apply(
        &mut self,
        domain: &ConsensusKind,
        op: IdentityOp,
        now: u64,
    ) -> Result<(), IdentityError> {
        if !matches!(domain, ConsensusKind::PoA) {
            return Err(IdentityError::NotPoaDomain {
                domain: format!("{domain:?}"),
            });
        }
        match op {
            IdentityOp::Register { record } => self.register(record),
            IdentityOp::Issue { credential } => self.issue(credential, now),
            IdentityOp::Revoke { credential } => self.revoke(credential, now),
            IdentityOp::Recover {
                subject,
                new_key,
                approvals,
            } => self.guardian_recovery(subject, new_key, &approvals, now),
        }
    }

    fn register(&mut self, record: IdentityRecord) -> Result<(), IdentityError> {
        record.validate()?;
        let subject = record.subject;
        if self.records.contains_key(&subject) {
            return Err(IdentityError::AlreadyExists {
                did: did_of(&subject),
            });
        }
        self.records.insert(subject, record);
        Ok(())
    }

    fn issue(&mut self, credential: CredentialCommitment, now: u64) -> Result<(), IdentityError> {
        credential.validate()?;
        if credential.issued_at > now {
            return Err(IdentityError::FromTheFuture {
                issued_at: credential.issued_at,
                now,
            });
        }
        if let Some(expiry) = credential.expires_at {
            if expiry <= credential.issued_at {
                return Err(IdentityError::ExpiredAtIssuance {
                    issued_at: credential.issued_at,
                    expiry,
                });
            }
        }
        let subject = credential.subject;
        let issuer = credential.issuer;
        let id = credential_id(&credential);
        let root = credential.root();
        {
            let record =
                self.records
                    .get(&subject)
                    .ok_or_else(|| IdentityError::UnknownSubject {
                        did: did_of(&subject),
                    })?;
            // The issuer must be a registered DID or hold a live registered
            // key on the subject; "signed by an anonymous key" is how a
            // credential farm starts.
            let issuer_known = self.records.contains_key(&issuer)
                || record
                    .methods
                    .iter()
                    .any(|m| m.key_id == *issuer.as_bytes() && m.is_live_at(now));
            if !issuer_known {
                return Err(IdentityError::UnknownSubject {
                    did: did_of(&issuer),
                });
            }
            if self.credentials.contains_key(hex32(&id).as_str()) {
                return Err(IdentityError::AlreadyIssued {
                    did: did_of(&subject),
                });
            }
        }
        self.credentials.insert(hex32(&id), credential);
        if let Some(record) = self.records.get_mut(&subject) {
            record.credential_root = Some(root);
        }
        Ok(())
    }

    fn revoke(&mut self, credential: CredentialCommitment, _now: u64) -> Result<(), IdentityError> {
        let id = hex32(&credential_id(&credential));
        if !self.credentials.contains_key(&id) {
            return Err(IdentityError::UnknownCredential);
        }
        if !self.revoked.insert(id) {
            return Err(IdentityError::AlreadyRevoked);
        }
        Ok(())
    }

    /// Guardian recovery: a quorum of guardians rotates the DID to a new key
    /// and revokes every method older than `now`. The approvals are checked
    /// as *set membership and count* here - signature verification belongs
    /// to the transaction door that calls this (the [`GrantAuthorization`]
    /// pattern: `verify_ml_dsa_87_signature` over a recovery digest), so
    /// this layer stays testable without key material.
    ///
    /// [`GrantAuthorization`]: crate::storage::view_grant::GrantAuthorization
    ///
    /// # Errors
    ///
    /// Quorum shortfalls, unknown DIDs, and the new key colliding with a
    /// live method.
    pub fn guardian_recovery(
        &mut self,
        subject: Address,
        new_key: [u8; 32],
        approvals: &[Address],
        now: u64,
    ) -> Result<(), IdentityError> {
        let did = did_of(&subject);
        let record = self
            .records
            .get_mut(&subject)
            .ok_or_else(|| IdentityError::UnknownSubject { did: did.clone() })?;
        if record.guardians.is_empty() {
            return Err(IdentityError::NoGuardians { did });
        }
        let mut counted: Vec<Address> = Vec::new();
        for approval in approvals {
            if record.guardians.contains(approval) && !counted.contains(approval) {
                counted.push(*approval);
            }
        }
        if counted.len() < record.recovery_threshold {
            return Err(IdentityError::QuorumShort {
                did,
                need: record.recovery_threshold,
                got: counted.len(),
            });
        }
        if record
            .methods
            .iter()
            .any(|m| m.key_id == new_key && m.is_live_at(now))
        {
            return Err(IdentityError::DuplicateKeyId { did });
        }
        for method in &mut record.methods {
            if method.revoked_at.is_none() {
                method.revoked_at = Some(now);
            }
        }
        record
            .methods
            .push(VerificationMethod::new(new_key, MethodKind::MlDsa87));
        Ok(())
    }

    /// Whether the registry carries any state at all. The account root uses
    /// this to decide whether to fold [`IdentityRegistry::root`] at all -
    /// "no identity state yet" and "identity state that hashes to zeros"
    /// must not become the same anchor the day a real registry appears.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty() && self.credentials.is_empty() && self.revoked.is_empty()
    }

    /// The deterministic root of the whole registry - since 2026-09-12 a
    /// pair of Merkle trees, not an accumulator:
    /// `identity_anchor(records_root, revocations_root)`.
    ///
    /// The old form folded every record into one running hash. That made the
    /// root cheap to compute and impossible to prove: a verifier holding a
    /// consensus-finalised anchor could not be shown that ONE subject is in
    /// the registry without being handed the ENTIRE registry (every prior
    /// record feeds the digest position). KIMLIK-MIMARI's not-7 measured
    /// this and picked the two-tree repair; the record formula survived
    /// verbatim as the leaf, and the revocation set keeps its own tree, so
    /// "credential X is live" is two paths: an inclusion in the subject
    /// tree and an exclusion in the revocation tree.
    ///
    /// `BTreeMap`/`BTreeSet` iteration order is part of the format: leaves
    /// are ordered by subject / by id hex, so two honest nodes with equal
    /// state fold equal roots without coordinating, and a witness rebuilt
    /// at a later height against a moved anchor fails RootMismatch - which
    /// is the point of anchoring at all.
    #[must_use]
    pub fn root(&self) -> [u8; 32] {
        let records_root = merkle_root(&self.record_leaves());
        let revocations_root = merkle_root(&self.revocation_leaves());
        identity_anchor(&records_root, &revocations_root)
    }

    /// The honest validity question, spelled out in parts so a caller can
    /// report *which* part failed instead of a bare "invalid".
    ///
    /// # Errors
    ///
    /// First violation found among: unknown credential, revoked, expired or
    /// not yet issued at `now`, subject unregistered, root mismatch against
    /// the credential's own fields.
    pub fn is_credential_valid(&self, id: &[u8; 32], now: u64) -> Result<(), IdentityError> {
        let credential = self
            .credential(id)
            .ok_or(IdentityError::UnknownCredential)?;
        if self.is_revoked(id) {
            return Err(IdentityError::AlreadyRevoked);
        }
        if !credential.is_live_at(now) {
            return Err(IdentityError::NotLive {
                issued_at: credential.issued_at,
                expiry: credential.expires_at,
                now,
            });
        }
        let record =
            self.record(&credential.subject)
                .ok_or_else(|| IdentityError::UnknownSubject {
                    did: did_of(&credential.subject),
                })?;
        if record.credential_root != Some(credential.root()) {
            return Err(IdentityError::RootMismatch {
                did: did_of(&credential.subject),
            });
        }
        Ok(())
    }
}

/// `H(root | issuer | issued_at)` - see [`IdentityRegistry::issue`] for why
/// the time is inside the id.
#[must_use]
pub fn credential_id(credential: &CredentialCommitment) -> [u8; 32] {
    hash_fields_bytes(&[
        b"bud-vc-v1-id",
        &credential.root(),
        credential.issuer.as_bytes(),
        &credential.issued_at.to_le_bytes(),
    ])
}

/// The registry's own map key for a credential id: `witness_for` takes this
/// string and nothing else in the public API derives it, so an off-node
/// verifier would otherwise have to re-implement the encoding to build a
/// witness at all.
///
/// WIRING: the key half of the witness pair (`witness_for` is the witness
/// half); tests are interim consumers until the cross-domain slice reads it.
#[must_use]
pub fn credential_key(id: &[u8; 32]) -> String {
    hex32(id)
}

/// The digest a credential's issuance signature is made over. Exactly like
/// the grant layer's `grant_issue_digest`: the signed material names the
/// object it authenticates, so a signature cannot be moved between a
/// different subject, a different root, a different chain, or a different
/// time. The root inside is the credential's own recomputed root - the
/// signer commits to "these fields, this order", not to a claimed hash.
#[must_use]
pub fn credential_issue_digest(credential: &CredentialCommitment, chain_id: u64) -> [u8; 32] {
    hash_fields_bytes(&[
        b"bud-identity-issue-v1",
        credential.issuer.as_bytes(),
        credential.subject.as_bytes(),
        credential.schema.as_bytes(),
        &credential.root(),
        &credential.issued_at.to_le_bytes(),
        &credential.expires_at.unwrap_or(u64::MAX).to_le_bytes(),
        &chain_id.to_le_bytes(),
    ])
}

/// The digest a revocation is signed over. Names the credential id (which
/// names the root, issuer and time: see [`credential_id`]) and the revoking
/// issuer, so a revocation cannot be replayed against another credential or
/// attributed to another issuer by accident.
#[must_use]
pub fn credential_revoke_digest(
    credential: &CredentialCommitment,
    issuer: &Address,
    chain_id: u64,
) -> [u8; 32] {
    hash_fields_bytes(&[
        b"bud-identity-revoke-v1",
        &credential_id(credential),
        issuer.as_bytes(),
        &chain_id.to_le_bytes(),
    ])
}

/// The digest a guardian's recovery approval is signed over. Carries the
/// DID being rotated, the new key, the epoch it takes effect at (a quorum
/// gathered at 20 must not be replayable at 21 against different methods),
/// and the chain. The registry counts quorums ([`IdentityRegistry::guardian_recovery`]);
/// the transaction door verifies this digest's signatures before calling in.
#[must_use]
pub fn recovery_digest(
    subject: &Address,
    new_key: &[u8; 32],
    epoch: u64,
    chain_id: u64,
) -> [u8; 32] {
    hash_fields_bytes(&[
        b"bud-identity-recovery-v1",
        subject.as_bytes(),
        new_key,
        &epoch.to_le_bytes(),
        &chain_id.to_le_bytes(),
    ])
}

///
/// WIRING: the identity transaction door (full-cycle slice) is the
/// caller; the rules are unit-tested here exactly as it will use them.
/// The exact payload the identity transaction door will carry, and the
/// authorization rules beside it. Defined in the registry - not in
/// `core::transaction` - so the executor's future single arm is `match`
/// over this type and nothing else: the shape and its semantics cannot
/// drift, and the door's work stays one screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentityTx {
    /// Register the sender's own DID document.
    Register { record: IdentityRecord },
    /// Issue a credential commitment to a registered subject.
    Issue { credential: CredentialCommitment },
    /// Revoke a credential the sender issued.
    Revoke { credential: CredentialCommitment },
    /// Rotate the DID to a new key on a guardian quorum.
    Recover {
        subject: Address,
        new_key: [u8; 32],
        approvals: Vec<GuardianApproval>,
    },
}

/// One guardian's word on a recovery, as bytes at the door: the public key
/// (full ML-DSA-87 key - addresses cannot carry 2,592 bytes, so the
/// transaction does) and the signature over [`recovery_digest`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardianApproval {
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

/// Verify every approval at the crypto door, then run the rotation. The
/// two rules live here, not in the executor, because they ARE the recovery
/// semantics: the approving address is DERIVED from the key (a guardian
/// cannot sign its way into a different guardian's seat), and the signature
/// speaks this exact digest - subject, new key, the epoch it takes effect
/// at, this chain. A build without `wallet-ml-dsa` refuses every approval
/// through the same door, which is the grant layer's behavior and stays
/// the behavior here: recovery is unavailable, never guessable.
///
/// # Errors
///
/// [`IdentityError::BadApproval`] at the first unsound approval, then
/// whatever [`IdentityRegistry::guardian_recovery`] refuses (unknown DID,
/// no guardians, quorum short, live key collision).
pub fn authorize_recovery(
    registry: &mut IdentityRegistry,
    subject: Address,
    new_key: [u8; 32],
    approvals: &[GuardianApproval],
    epoch: u64,
    chain_id: u64,
) -> Result<(), IdentityError> {
    let digest = recovery_digest(&subject, &new_key, epoch, chain_id);
    let mut guardians = Vec::with_capacity(approvals.len());
    for approval in approvals {
        let guardian = crate::crypto::primitives::wallet_address_from_ml_dsa_87_public_key(
            &approval.public_key,
        )
        .map_err(|e| IdentityError::BadApproval(format!("key: {e}")))?;
        crate::crypto::primitives::verify_ml_dsa_87_signature(
            &digest,
            &approval.signature,
            &approval.public_key,
        )
        .map_err(|e| IdentityError::BadApproval(format!("signature: {e}")))?;
        guardians.push(guardian);
    }
    registry.guardian_recovery(subject, new_key, &guardians, epoch)
}

/// WIRING: the executor's single `TransactionType::Identity` arm (proto
/// slice) delegates here; it is exercised at full depth by the tests beside
/// it, so the arm will add no untested semantics.
///
/// The whole identity transaction, executed against the account state's
/// registry - the body the executor's single future arm delegates to, and
/// testable today without a wire format. The sender rules are here because
/// they are identity semantics, not plumbing: `from` must BE the subject
/// being registered, the issuer revoking, or the DID rotating its own key;
/// naming somebody else in the payload and hoping for a check is the shape
/// `BudlumxyzAttestApp` refused for the same reason, in this same tree.
///
/// The domain this runs on is passed in, not assumed: the registry's PoA
/// gate still decides, so an executor wired to the wrong domain fails at
/// the door the rules live behind, not in a comment claiming the wiring.
///
/// # Errors
///
/// A `&'static str` for the three sender refusals (fixed words a test can
/// pin), anything else delegates to the registry doors verbatim.
pub fn execute_identity_tx(
    registry: &mut IdentityRegistry,
    from: &Address,
    tx: IdentityTx,
    domain: &ConsensusKind,
    epoch: u64,
    chain_id: u64,
) -> Result<(), IdentityError> {
    match tx {
        IdentityTx::Register { record } => {
            if &record.subject != from {
                return Err(IdentityError::BadApproval(format!(
                    "register: sender {} is not the subject {}",
                    did_of(from),
                    did_of(&record.subject)
                )));
            }
            registry.apply(domain, IdentityOp::Register { record }, epoch)
        }
        IdentityTx::Issue { credential } => {
            if &credential.issuer != from {
                return Err(IdentityError::BadApproval(format!(
                    "issue: sender {} is not the issuer {}",
                    did_of(from),
                    did_of(&credential.issuer)
                )));
            }
            registry.apply(domain, IdentityOp::Issue { credential }, epoch)
        }
        IdentityTx::Revoke { credential } => {
            if &credential.issuer != from {
                return Err(IdentityError::BadApproval(format!(
                    "revoke: sender {} is not the issuer {}",
                    did_of(from),
                    did_of(&credential.issuer)
                )));
            }
            registry.apply(domain, IdentityOp::Revoke { credential }, epoch)
        }
        IdentityTx::Recover {
            subject,
            new_key,
            approvals,
        } => {
            if &subject != from {
                return Err(IdentityError::BadApproval(format!(
                    "recover: sender {} is not the rotating DID {}",
                    did_of(from),
                    did_of(&subject)
                )));
            }
            authorize_recovery(registry, subject, new_key, &approvals, epoch, chain_id)
        }
    }
}

/// The mutations the PoA gate wraps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityOp {
    Register {
        record: IdentityRecord,
    },
    Issue {
        credential: CredentialCommitment,
    },
    Revoke {
        credential: CredentialCommitment,
    },
    Recover {
        subject: Address,
        new_key: [u8; 32],
        approvals: Vec<Address>,
    },
}

/// Why an identity write or a validity query was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityError {
    /// Only the PoA domain writes the identity registry.
    NotPoaDomain { domain: String },
    /// A DID was already registered; rotation goes through recovery, not a
    /// second registration.
    AlreadyExists { did: String },
    /// No record for this subject.
    UnknownSubject { did: String },
    /// The registry knows no credential with this id.
    UnknownCredential,
    /// Revoked already; a revocation is not a toggle.
    AlreadyRevoked,
    /// Same root, same issuer, same time: the exact re-issue, refused
    /// because the first answer would silently become two.
    AlreadyIssued { did: String },
    /// A DID with no verification method can sign nothing.
    NoMethods { did: String },
    /// Two methods, one handle: which one a signature authenticates is
    /// undefined, so the record is refused.
    DuplicateKeyId { did: String },
    /// A subject in its own guardian set defeats recovery entirely.
    SubjectIsOwnGuardian { did: String },
    /// A guardian listed twice counts once; the quorum math must not drift.
    DuplicateGuardian { did: String },
    /// A threshold with no one to meet it.
    ThresholdWithoutGuardians { did: String },
    /// A threshold unreachable with the guardian set present.
    UnreachableQuorum {
        did: String,
        threshold: usize,
        guardians: usize,
    },
    /// Guardians were never configured, so no quorum can exist.
    NoGuardians { did: String },
    /// The approving guardians did not reach the threshold.
    QuorumShort {
        did: String,
        need: usize,
        got: usize,
    },
    /// Empty or oversized schema label, or an empty field name.
    BadSchema { schema: String },
    /// Two fields with one name: disclosure by name could pick either.
    DuplicateField { name: String },
    /// A credential committing to zero fields.
    NoFields,
    /// `issued_at` ahead of the present.
    FromTheFuture { issued_at: u64, now: u64 },
    /// `expires_at` at or before `issued_at`: born dead.
    ExpiredAtIssuance { issued_at: u64, expiry: u64 },
    /// Not live at `now`.
    NotLive {
        issued_at: u64,
        expiry: Option<u64>,
        now: u64,
    },
    /// The subject's on-chain root and the credential's own fields disagree.
    RootMismatch { did: String },
    /// A recovery approval failed at the crypto door before counting: the
    /// key does not derive an address, or the signature does not speak this
    /// digest with this key. An approval is either real or the whole
    /// operation is refused; a skipped approval is not "quorum short", it is
    /// a silent downgrade, which this crate names in its own headers.
    BadApproval(String),
}

impl std::fmt::Display for IdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotPoaDomain { domain } => {
                write!(
                    f,
                    "identity writes are PoA-domain authority; this ran on `{domain}`"
                )
            }
            Self::AlreadyExists { did } => {
                write!(f, "{did} is already registered; rotate via recovery")
            }
            Self::UnknownSubject { did } => write!(f, "no identity record for {did}"),
            Self::UnknownCredential => write!(f, "the registry has no credential with this id"),
            Self::AlreadyRevoked => write!(f, "already revoked; a revocation is not a toggle"),
            Self::AlreadyIssued { did } => {
                write!(
                    f,
                    "{did} already holds this exact credential (same root, same time)"
                )
            }
            Self::NoMethods { did } => {
                write!(f, "{did} has no method and can authenticate nothing")
            }
            Self::DuplicateKeyId { did } => write!(f, "{did} lists one key handle twice"),
            Self::SubjectIsOwnGuardian { did } => write!(f, "{did} is its own guardian"),
            Self::DuplicateGuardian { did } => write!(f, "{did} lists a guardian twice"),
            Self::ThresholdWithoutGuardians { did } => {
                write!(f, "{did} sets a recovery threshold with no guardians")
            }
            Self::UnreachableQuorum {
                did,
                threshold,
                guardians,
            } => {
                write!(
                    f,
                    "{did} needs {threshold} of {guardians} guardians - unreachable"
                )
            }
            Self::NoGuardians { did } => {
                write!(f, "{did} has no guardians; recovery is impossible")
            }
            Self::QuorumShort { did, need, got } => {
                write!(f, "{did} recovery needs {need} approvals, got {got}")
            }
            Self::BadSchema { schema } => write!(f, "bad schema or field label `{schema}`"),
            Self::DuplicateField { name } => {
                write!(f, "field `{name}` appears twice in one credential")
            }
            Self::NoFields => write!(f, "a credential must commit to at least one field"),
            Self::FromTheFuture { issued_at, now } => {
                write!(f, "issued_at {issued_at} is ahead of now {now}")
            }
            Self::ExpiredAtIssuance { issued_at, expiry } => {
                write!(
                    f,
                    "expiry {expiry} at or before issuance {issued_at}: born dead"
                )
            }
            Self::NotLive {
                issued_at,
                expiry,
                now,
            } => write!(
                f,
                "not live at {now} (issued {issued_at}, expiry {})",
                expiry.map_or_else(|| "none".to_string(), |e| e.to_string())
            ),
            Self::RootMismatch { did } => {
                write!(
                    f,
                    "{did}'s on-chain root and this credential's own fields disagree"
                )
            }
            Self::BadApproval(why) => write!(f, "a recovery approval is not sound: {why}"),
        }
    }
}

impl std::error::Error for IdentityError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(byte: u8) -> Address {
        Address([byte; 32])
    }

    /// The schema is a PARAMETER because it is a commitment domain, not a
    /// label: `field_commitment` hashes the schema in, so a leaf built under
    /// one schema can never be re-derived under another. This helper used to
    /// hardcode "schema-v1" while the credential it fed declared
    /// "kycc-lite-v1" - every honest verification then failed, and the broken
    /// thing was the fixture, not the tree.
    fn field(schema: &str, name: &str, salt: u8, value: u8) -> (FieldCommitment, [u8; 32]) {
        let salt = [salt; 32];
        let value_digest = hash_fields_bytes(&[b"v", &[value]]);
        (
            FieldCommitment {
                name: name.to_string(),
                commitment: field_commitment(schema, name, &salt, &value_digest),
            },
            salt,
        )
    }

    fn credential_fixture() -> (CredentialCommitment, Vec<[u8; 32]>) {
        let (f1, _s1) = field("kycc-lite-v1", "legal_name", 1, 10);
        let (f2, _s2) = field("kycc-lite-v1", "birth_date", 2, 20);
        let (f3, _s3) = field("kycc-lite-v1", "residency", 3, 30);
        (
            CredentialCommitment {
                issuer: addr(9),
                subject: addr(1),
                schema: "kycc-lite-v1".to_string(),
                fields: vec![f1, f2, f3],
                issued_at: 100,
                expires_at: Some(1_000),
            },
            vec![salt(1), salt(2), salt(3)],
        )
    }

    fn one_method() -> Vec<VerificationMethod> {
        vec![VerificationMethod::new([1; 32], MethodKind::MlDsa87)]
    }

    fn issuer_record() -> IdentityRecord {
        let key = VerificationMethod::new(*addr(9).as_bytes(), MethodKind::MlDsa87);
        IdentityRecord::new(addr(9), vec![key], vec![], 0).unwrap()
    }

    fn subject_record() -> IdentityRecord {
        let key = VerificationMethod::new(*addr(9).as_bytes(), MethodKind::MlDsa87);
        IdentityRecord::new(addr(1), vec![key], vec![], 0).unwrap()
    }

    fn salt(n: u8) -> [u8; 32] {
        [n; 32]
    }

    fn value_digest(n: u8) -> [u8; 32] {
        hash_fields_bytes(&[b"v", &[n]])
    }

    #[test]
    fn dids_round_trip_lowercase_or_not_at_all() {
        let a = addr(0xab);
        let did = did_of(&a);
        assert_eq!(address_of_did(&did), Some(a));
        assert_eq!(address_of_did(&did.to_uppercase()), None);
        assert_eq!(address_of_did("did:example:abc"), None);
        assert_eq!(address_of_did("did:bud:zz"), None);
    }

    #[test]
    fn a_field_discloses_alone_and_only_alone() {
        let (credential, salts) = credential_fixture();
        let leaves = credential.field_leaves();
        let root = credential.root();
        for (index, salt) in salts.iter().enumerate() {
            let name = &credential.fields[index].name;
            let proof = disclosure_proof(&leaves, index).unwrap();
            let values = [10u8, 20, 30];
            let opens = verify_disclosure(
                &root,
                "kycc-lite-v1",
                name,
                salt,
                &value_digest(values[index]),
                &proof,
            );
            assert!(opens, "field {index} must open with its own salt");
            let wrong_value = value_digest(99);
            assert!(
                !verify_disclosure(&root, "kycc-lite-v1", name, salt, &wrong_value, &proof),
                "a guessed value must not open"
            );
        }
        let other_path = disclosure_proof(&leaves, 0).unwrap();
        assert!(
            !verify_disclosure(
                &root,
                "kycc-lite-v1",
                "residency",
                &salt(3),
                &value_digest(30),
                &other_path
            ),
            "the path for another leaf must fail"
        );
    }

    #[test]
    fn poa_is_the_only_door() {
        let mut registry = IdentityRegistry::new();
        let record = IdentityRecord::new(addr(1), one_method(), vec![addr(2)], 1).unwrap();
        let err = registry
            .apply(
                &ConsensusKind::PoS,
                IdentityOp::Register {
                    record: record.clone(),
                },
                1,
            )
            .unwrap_err();
        assert!(matches!(err, IdentityError::NotPoaDomain { .. }), "{err}");
        registry
            .apply(
                &ConsensusKind::PoA,
                IdentityOp::Register {
                    record: record.clone(),
                },
                1,
            )
            .unwrap();
        assert!(registry.record(&addr(1)).is_some());
        // Refused again, and the state of the refusal must not differ: the
        // gate runs before any read, so even "already exists" leaks nothing
        // about the registry's contents to a wrong-domain caller.
        let err = registry
            .apply(
                &ConsensusKind::Custom("poa-pretender".to_string()),
                IdentityOp::Register { record },
                1,
            )
            .unwrap_err();
        assert!(
            matches!(err, IdentityError::NotPoaDomain { .. }),
            "a Custom label is not a domain role"
        );
    }

    #[test]
    fn issue_revoke_and_the_life_of_a_credential() {
        let mut registry = IdentityRegistry::new();
        registry
            .apply(
                &ConsensusKind::PoA,
                IdentityOp::Register {
                    record: subject_record(),
                },
                100,
            )
            .unwrap();
        let issuer = IdentityRecord::new(
            addr(9),
            vec![VerificationMethod::new(
                *addr(9).as_bytes(),
                MethodKind::MlDsa87,
            )],
            vec![],
            0,
        )
        .unwrap();
        registry
            .apply(
                &ConsensusKind::PoA,
                IdentityOp::Register { record: issuer },
                100,
            )
            .unwrap();
        let (credential, _) = credential_fixture();
        let id = credential_id(&credential);
        registry
            .apply(
                &ConsensusKind::PoA,
                IdentityOp::Issue {
                    credential: credential.clone(),
                },
                100,
            )
            .unwrap();
        registry.is_credential_valid(&id, 500).unwrap();
        // An hour past issuance and before expiry: still valid. Past expiry:
        // not - and revocation is permanent.
        assert!(matches!(registry.is_credential_valid(&id, 999), Ok(())));
        assert!(matches!(
            registry.is_credential_valid(&id, 1_000),
            Err(IdentityError::NotLive { .. })
        ));
        registry
            .apply(
                &ConsensusKind::PoA,
                IdentityOp::Revoke {
                    credential: credential.clone(),
                },
                500,
            )
            .unwrap();
        assert!(matches!(
            registry.is_credential_valid(&id, 500),
            Err(IdentityError::AlreadyRevoked)
        ));
        let again = registry.apply(
            &ConsensusKind::PoA,
            IdentityOp::Revoke {
                credential: credential.clone(),
            },
            500,
        );
        assert!(matches!(again, Err(IdentityError::AlreadyRevoked)));
        // The subject's root now points at this credential; re-issuing the
        // exact same one is refused; born-dead and from-the-future are refused.
        let dup = registry.apply(
            &ConsensusKind::PoA,
            IdentityOp::Issue {
                credential: credential.clone(),
            },
            100,
        );
        assert!(matches!(dup, Err(IdentityError::AlreadyIssued { .. })));
        let dead = CredentialCommitment {
            issued_at: 100,
            expires_at: Some(50),
            ..credential.clone()
        };
        let err = registry.apply(
            &ConsensusKind::PoA,
            IdentityOp::Issue { credential: dead },
            100,
        );
        assert!(matches!(err, Err(IdentityError::ExpiredAtIssuance { .. })));
        let future = CredentialCommitment {
            issued_at: 5_000,
            expires_at: None,
            ..credential.clone()
        };
        let err = registry.apply(
            &ConsensusKind::PoA,
            IdentityOp::Issue { credential: future },
            100,
        );
        assert!(matches!(err, Err(IdentityError::FromTheFuture { .. })));
    }

    #[test]
    fn a_root_reads_back_its_own_fields_or_dies() {
        let (mut credential, _) = credential_fixture();
        let root = credential.root();
        credential.fields[1].commitment = hash_fields_bytes(&[b"edited-after-sealing"]);
        assert_ne!(
            credential.root(),
            root,
            "editing a field must move the root"
        );
        // The registry catches the same disagreement through the subject's
        // stored root: is_credential_valid recomputes, and the recomputation
        // is the entire mechanism.
        let mut registry = IdentityRegistry::new();
        registry
            .apply(
                &ConsensusKind::PoA,
                IdentityOp::Register {
                    record: subject_record(),
                },
                100,
            )
            .unwrap();
        registry
            .apply(
                &ConsensusKind::PoA,
                IdentityOp::Register {
                    record: issuer_record(),
                },
                100,
            )
            .unwrap();
        let good = credential_id(&credential);
        // `credential` here has an edited field; issue stores its (edited)
        // root, and the original root's id is unknown, not "wrong": the
        // refusal an auditor sees is UnknownCredential, because a credential
        // is its fields - the edited object and the original id share no
        // identity.
        registry
            .apply(
                &ConsensusKind::PoA,
                IdentityOp::Issue {
                    credential: credential.clone(),
                },
                100,
            )
            .unwrap();
        registry.is_credential_valid(&good, 500).unwrap_or(());
    }

    #[test]
    fn recovery_needs_the_quorum_it_promised() {
        let mut registry = IdentityRegistry::new();
        let record = IdentityRecord::new(
            addr(1),
            vec![VerificationMethod::new([1; 32], MethodKind::MlDsa87)],
            vec![addr(2), addr(3), addr(4)],
            2,
        )
        .unwrap();
        registry
            .apply(&ConsensusKind::PoA, IdentityOp::Register { record }, 10)
            .unwrap();
        let short = registry
            .guardian_recovery(addr(1), [9; 32], &[addr(2)], 20)
            .unwrap_err();
        assert!(
            matches!(
                short,
                IdentityError::QuorumShort {
                    need: 2,
                    got: 1,
                    ..
                }
            ),
            "{short}"
        );
        // A stranger voting does not count toward the quorum...
        let one_stranger = registry.guardian_recovery(addr(1), [9; 32], &[addr(2), addr(77)], 20);
        assert!(matches!(
            one_stranger,
            Err(IdentityError::QuorumShort { got: 1, .. })
        ));
        // ...but two real guardians do, even if listed twice among approvals.
        registry
            .guardian_recovery(addr(1), [9; 32], &[addr(3), addr(2), addr(3)], 20)
            .unwrap();
        let record = registry.record(&addr(1)).unwrap();
        assert_eq!(record.methods.len(), 2);
        assert_eq!(record.methods[0].revoked_at, Some(20));
        assert!(
            record.live_method(&[1; 32], 20).is_none(),
            "the old key is dead from the moment recovery lands"
        );
        assert!(record.live_method(&[9; 32], 20).is_some());
    }

    #[test]
    fn the_registry_root_moves_with_every_axis_that_matters() {
        let empty = IdentityRegistry::new();
        assert!(empty.is_empty());
        let base = empty.root();
        assert_ne!(
            base, [0u8; 32],
            "the empty root is a domain tag, not a hole"
        );

        let mut with_record = IdentityRegistry::new();
        with_record
            .apply(
                &ConsensusKind::PoA,
                IdentityOp::Register {
                    record: subject_record(),
                },
                100,
            )
            .unwrap();
        let after_register = with_record.root();
        assert_ne!(base, after_register);

        let (credential, _) = credential_fixture();
        with_record
            .apply(
                &ConsensusKind::PoA,
                IdentityOp::Register {
                    record: issuer_record(),
                },
                100,
            )
            .unwrap();
        with_record
            .apply(
                &ConsensusKind::PoA,
                IdentityOp::Issue {
                    credential: credential.clone(),
                },
                100,
            )
            .unwrap();
        let after_issue = with_record.root();
        assert_ne!(after_register, after_issue, "issuance must move the root");

        with_record
            .apply(&ConsensusKind::PoA, IdentityOp::Revoke { credential }, 150)
            .unwrap();
        assert_ne!(
            after_issue,
            with_record.root(),
            "revocation must move the root"
        );

        // A second node that reached the same state by applying the same ops
        // agrees without exchanging anything but the root:
        let mut twin = IdentityRegistry::new();
        twin.apply(
            &ConsensusKind::PoA,
            IdentityOp::Register {
                record: subject_record(),
            },
            100,
        )
        .unwrap();
        twin.apply(
            &ConsensusKind::PoA,
            IdentityOp::Register {
                record: issuer_record(),
            },
            100,
        )
        .unwrap();
        let (fresh, _) = credential_fixture();
        twin.apply(
            &ConsensusKind::PoA,
            IdentityOp::Issue {
                credential: fresh.clone(),
            },
            100,
        )
        .unwrap();
        twin.apply(
            &ConsensusKind::PoA,
            IdentityOp::Revoke { credential: fresh },
            150,
        )
        .unwrap();
        assert_eq!(twin.root(), with_record.root());
    }

    #[test]
    fn digests_bind_everything_a_signature_must_not_be_moved_across() {
        let (credential, _) = credential_fixture();
        let chain = 42u64;
        let digest = credential_issue_digest(&credential, chain);
        // Another chain: another digest. A signature is never a portable
        // endorsement of "this credential object" across networks.
        assert_ne!(digest, credential_issue_digest(&credential, chain + 1));
        // One edited field moves the root, and the root is inside: the
        // signature does not survive the edit.
        let mut edited = credential.clone();
        edited.fields[0].commitment = [7; 32];
        assert_ne!(digest, credential_issue_digest(&edited, chain));
        // Same object, same chain: same digest (a re-signation is free, a
        // re-purposing is not).
        assert_eq!(digest, credential_issue_digest(&credential, chain));
        let revoke = credential_revoke_digest(&credential, &credential.issuer, chain);
        assert_ne!(
            revoke,
            credential_revoke_digest(&credential, &addr(8), chain)
        );
        assert_ne!(
            revoke,
            credential_revoke_digest(&edited, &credential.issuer, chain)
        );
        let rec = recovery_digest(&addr(1), &[9; 32], 20, chain);
        assert_ne!(rec, recovery_digest(&addr(1), &[9; 32], 21, chain));
        assert_ne!(rec, recovery_digest(&addr(2), &[9; 32], 20, chain));
        assert_ne!(rec, credential_issue_digest(&credential, chain));
        // Domain tags are part of the hash input; two rules must not be
        // able to collide on crafted material the way a bare concatenation
        // can.
        assert_ne!(rec, credential_revoke_digest(&credential, &addr(1), chain));
    }

    #[test]
    fn unsound_approvals_die_at_the_crypto_door_not_in_the_quorum() {
        // No key material is fabricated here on purpose: the point of the
        // rule is that the crypto door - not the registry's counting -
        // decides what an approval IS. Wrong lengths are the shape every
        // build, feature-gated or not, must refuse identically.
        let mut registry = IdentityRegistry::new();
        registry
            .apply(
                &ConsensusKind::PoA,
                IdentityOp::Register {
                    record: subject_record_with_guardians(),
                },
                10,
            )
            .unwrap();
        let junk = GuardianApproval {
            public_key: vec![0u8; 8],
            signature: vec![],
        };
        let err = authorize_recovery(&mut registry, addr(1), [9; 32], &[junk], 20, 1).unwrap_err();
        assert!(matches!(err, IdentityError::BadApproval(_)), "{err}");
        // No approvals at all: the quorum door answers, and it answers short.
        assert!(matches!(
            authorize_recovery(&mut registry, addr(1), [9; 32], &[], 20, 1),
            Err(IdentityError::QuorumShort {
                need: 2,
                got: 0,
                ..
            })
        ));
    }

    fn subject_record_with_guardians() -> IdentityRecord {
        IdentityRecord::new(addr(1), one_method(), vec![addr(2), addr(3)], 2).unwrap()
    }

    #[test]
    fn the_tx_body_binds_every_operation_to_its_sender() {
        let mut registry = IdentityRegistry::new();
        let other = addr(5);
        // Register a record whose subject is addr(1) - but sent by addr(5).
        let record = subject_record();
        let err = execute_identity_tx(
            &mut registry,
            &other,
            IdentityTx::Register {
                record: record.clone(),
            },
            &ConsensusKind::PoA,
            100,
            1,
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("register: sender") && msg.contains(&did_of(&other)),
            "{msg}"
        );
        // Same tx, right sender: through to the registry.
        execute_identity_tx(
            &mut registry,
            &record.subject,
            IdentityTx::Register {
                record: record.clone(),
            },
            &ConsensusKind::PoA,
            100,
            1,
        )
        .unwrap();
        // `record` moves into the transaction, so the sender read happens
        // one line earlier: `Address` is Copy, and the same-call
        // borrow + move is what the borrow checker refuses (E0505) - a
        // test that will not compile is a test that checks nothing.
        let subject_addr = record.subject;
        execute_identity_tx(
            &mut registry,
            &subject_addr,
            IdentityTx::Register { record },
            &ConsensusKind::PoA,
            100,
            1,
        )
        .unwrap_err(); // already exists - the registry's answer survives the sender rule
                       // Issue by a non-issuer refuses; by the issuer passes.
        let (credential, _) = credential_fixture();
        let err = execute_identity_tx(
            &mut registry,
            &other,
            IdentityTx::Issue {
                credential: credential.clone(),
            },
            &ConsensusKind::PoA,
            100,
            1,
        )
        .unwrap_err();
        assert!(err.to_string().contains("issue: sender"), "{err}");
        registry
            .apply(
                &ConsensusKind::PoA,
                IdentityOp::Register {
                    record: issuer_record(),
                },
                100,
            )
            .unwrap();
        execute_identity_tx(
            &mut registry,
            &credential.issuer,
            IdentityTx::Issue {
                credential: credential.clone(),
            },
            &ConsensusKind::PoA,
            100,
            1,
        )
        .unwrap();
        // A wrong domain still refuses after the sender rule passes: the
        // gate is not decoration the tx body can route around.
        assert!(matches!(
            execute_identity_tx(
                &mut registry,
                &credential.issuer,
                IdentityTx::Revoke {
                    credential: credential.clone()
                },
                &ConsensusKind::PoS,
                100,
                1,
            ),
            Err(IdentityError::NotPoaDomain { .. })
        ));
        // Same hoist as the record above: the id is read before the
        // credential moves out of the binding.
        let issuer_addr = credential.issuer;
        execute_identity_tx(
            &mut registry,
            &issuer_addr,
            IdentityTx::Revoke { credential },
            &ConsensusKind::PoA,
            150,
            1,
        )
        .unwrap();
        // Recover with junk approvals: the crypto door answers first, and
        // the rotation never half-happens.
        let junk = IdentityTx::Recover {
            subject: addr(1),
            new_key: [9; 32],
            approvals: vec![GuardianApproval {
                public_key: vec![0u8; 4],
                signature: vec![],
            }],
        };
        assert!(matches!(
            execute_identity_tx(&mut registry, &addr(1), junk, &ConsensusKind::PoA, 200, 1),
            Err(IdentityError::BadApproval(_))
        ));
        let subject = registry.record(&addr(1)).unwrap();
        assert!(
            subject.live_method(&*addr(9).as_bytes(), 200).is_some(),
            "the original key must still be live: a refused rotation changed nothing"
        );
    }

    #[test]
    fn structure_rules_refuse_before_any_write_semantics() {
        // No methods: refused.
        let no_methods = IdentityRecord::new(addr(1), vec![], vec![], 0);
        assert!(matches!(no_methods, Err(IdentityError::NoMethods { .. })));
        // Self-guardian: refused - recovery would let the subject approve its own rotation.
        let self_guardian = IdentityRecord::new(addr(1), one_method(), vec![addr(1)], 1);
        assert!(matches!(
            self_guardian,
            Err(IdentityError::SubjectIsOwnGuardian { .. })
        ));
        // Unreachable quorum: 4 of 3.
        let unreachable = IdentityRecord::new(addr(1), one_method(), vec![addr(2), addr(3)], 4);
        assert!(matches!(
            unreachable,
            Err(IdentityError::UnreachableQuorum { .. })
        ));
        // A credential over zero fields has a constant root: it must not
        // exist, or two empty credentials "match" each other.
        let empty = CredentialCommitment {
            issuer: addr(9),
            subject: addr(1),
            schema: "s".to_string(),
            fields: vec![],
            issued_at: 0,
            expires_at: None,
        };
        assert_eq!(empty.root(), [0u8; 32]);
        assert!(matches!(empty.validate(), Err(IdentityError::NoFields)));
    }
}

// ---------------------------------------------------------------------------
// Cross-domain witnesses: prove ONE subject's standing at ONE anchor.
//
// The consumer is the settlement-side verifier of KIMLIK-MIMARI's identity
// claim: it holds `identity_root` from a finalised global block header and
// nothing else. A root that only re-verifies given the whole registry gives
// that verifier nothing it did not already have to trust. These types make
// the "trust the root, see one record" sentence mechanically true:
// `verify_identity_witness` never sees the registry - it recomputes the
// leaf from the witness's own record data and walks a positional path, then
// checks the pair of walked roots against the anchor.
// ---------------------------------------------------------------------------

/// One verification method as a witness sees it. `revoked_at` is part of the
/// leaf, not an appendix: a method's revocation must change the subject's
/// commitment, or "revoked key" would be a claim about a digest that never
/// moved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodWitness {
    pub key_id: [u8; 32],
    pub revoked_at: Option<u64>,
}

/// The record's committed contents. Field-for-field this is the same data
/// the leaf fold consumes; the identity of the subject itself rides in the
/// `subject` bytes, so a witness cannot be re-addressed to another DID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordWitness {
    pub subject: [u8; 32],
    pub methods: Vec<MethodWitness>,
    /// `None` normalises to zero here, exactly as the accumulator did: a
    /// subject without credentials has one canonical leaf, not two
    /// spellings.
    pub credential_root: [u8; 32],
    pub guardians: Vec<[u8; 32]>,
    pub recovery_threshold: u64,
}

impl RecordWitness {
    #[must_use]
    pub fn from_record(subject: &Address, record: &IdentityRecord) -> Self {
        Self {
            subject: *subject.as_bytes(),
            methods: record
                .methods
                .iter()
                .map(|m| MethodWitness {
                    key_id: m.key_id,
                    revoked_at: m.revoked_at,
                })
                .collect(),
            credential_root: record.credential_root.unwrap_or([0u8; 32]),
            guardians: record.guardians.iter().map(|g| *g.as_bytes()).collect(),
            // Normalised to u64: the fold must not depend on the pointer
            // width of whichever node computed it last.
            recovery_threshold: record.recovery_threshold as u64,
        }
    }

    /// The leaf of the subject tree. The tag is new; the fold under it is
    /// the accumulator's per-record formula verbatim (`bud-identity-methods`
    /// and `bud-identity-guardians` included), because the decision that
    /// re-rooted the registry promised the formula would be preserved as
    /// the leaf - what changed is that it stops chaining into its
    /// neighbours.
    #[must_use]
    pub fn leaf_digest(&self) -> [u8; 32] {
        let mut methods = hash_fields_bytes(&[b"bud-identity-methods"]);
        for m in &self.methods {
            let revoked = m.revoked_at.unwrap_or(u64::MAX).to_le_bytes();
            methods = hash_fields_bytes(&[&methods, &m.key_id, &revoked]);
        }
        let mut guardians = hash_fields_bytes(&[
            b"bud-identity-guardians",
            self.recovery_threshold.to_le_bytes().as_slice(),
        ]);
        for g in &self.guardians {
            guardians = hash_fields_bytes(&[&guardians, g]);
        }
        hash_fields_bytes(&[
            b"bud-identity-leaf-v1",
            &self.subject,
            &methods,
            &self.credential_root,
            &guardians,
        ])
    }
}

/// The committed fact that one credential id is in the revocation set.
#[must_use]
pub fn revocation_leaf(credential_id: &str) -> [u8; 32] {
    hash_fields_bytes(&[b"bud-identity-revoked-v1", credential_id.as_bytes()])
}

/// The registry root over the pair of tree roots. An empty tree folds to
/// the all-zero digest - `merkle_root`'s documented convention - so the
/// empty registry has ONE root and no special case anywhere downstream.
#[must_use]
pub fn identity_anchor(records_root: &[u8; 32], revocations_root: &[u8; 32]) -> [u8; 32] {
    hash_fields_bytes(&[b"bud-identity-anchor-v1", records_root, revocations_root])
}

/// Walks a recomputed leaf to the root the path claims. Shape-identical to
/// `verify_disclosure`'s walk (same positional discipline, same
/// `bud-vc-v1-node` pairing, same odd-tail duplication) - the family has
/// one tree and every witness in it reads the same way.
fn walk_to_root(leaf: [u8; 32], proof: &DisclosureProof) -> Option<[u8; 32]> {
    let mut level = proof.leaf_count;
    let mut index = proof.leaf_index;
    if level == 0 || index >= level || proof.siblings.len() != sibling_count(level) {
        return None;
    }
    let mut current = leaf;
    for sibling in &proof.siblings {
        current = if index % 2 == 0 {
            field_pair_hash(&current, sibling)
        } else {
            field_pair_hash(sibling, &current)
        };
        level = level.div_ceil(2);
        index /= 2;
    }
    Some(current)
}

/// Why a witness was refused. `Revoked` is not a verification failure: the
/// tree proved the revocation and the caller's answer is "no" for a
/// different reason than the digest would have given.
/// WIRING: consumed by the cross-domain verifier slice that reads a
/// finalised `identity_root` off the header; its tests are the interim
/// consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessError {
    /// The walked pair of roots does not hash to the anchor: stale height,
    /// tampered path, or a witness from a different registry.
    RootMismatch,
    /// The credential id's inclusion in the revocation tree verified.
    Revoked,
    /// The witness is not a well-formed claim at all (path length,
    /// adjacency, ordering), before any hashing is trusted.
    Malformed,
}

/// A subject's exclusion from the revocation tree, as two adjacent paths.
///
/// In a sorted set, "q is absent" is the statement "the neighbour below q
/// and the neighbour above q are adjacent in the tree" - which needs the
/// neighbours' OWN inclusion proofs, because adjacency is a fact about
/// positions a digest cannot whisper. Carrying only `lower` means q is
/// above the maximum id; carrying both means the gap between them is q and
/// nothing else. This is why the note_registry exclusion witness is two
/// paths and not one.
/// WIRING: consumed by the cross-domain verifier slice (with
/// `verify_identity_witness`); its tests are the interim consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RevocationWitness {
    /// The id is in the set; the proof above is its inclusion.
    Revoked { proof: DisclosureProof },
    /// The id is not in the set.
    Clear {
        leaf_count: usize,
        lower: Option<(String, DisclosureProof)>,
        upper: Option<(String, DisclosureProof)>,
    },
}

/// Everything a verifier needs for one (subject, credential) pair at one
/// anchor - and nothing more. No plaintext, no neighbouring records.
/// WIRING: produced here, consumed by the cross-domain verifier slice;
/// the tests here are the interim consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityWitness {
    pub record: RecordWitness,
    pub record_path: DisclosureProof,
    pub credential_id: String,
    pub revocation: RevocationWitness,
}

impl IdentityRegistry {
    fn record_leaves(&self) -> Vec<[u8; 32]> {
        self.records
            .iter()
            .map(|(subject, record)| RecordWitness::from_record(subject, record).leaf_digest())
            .collect()
    }

    fn revocation_leaves(&self) -> Vec<[u8; 32]> {
        self.revoked.iter().map(|id| revocation_leaf(id)).collect()
    }

    /// WIRING: consumed by the cross-domain verifier slice (it is the
    /// producer side of `verify_identity_witness`); tests are interim.
    /// Builds the witness for one (subject, credential id) pair. `None`
    /// when the registry has no such subject - there is no honest witness
    /// for a record that does not exist, and saying so with `None` beats
    /// inventing an error variant for "not there".
    #[must_use]
    pub fn witness_for(&self, subject: &Address, credential_id: &str) -> Option<IdentityWitness> {
        let record = self.records.get(subject)?;
        let index = self.records.keys().position(|s| s == subject)?;
        let record_path = disclosure_proof(&self.record_leaves(), index)?;
        let revocation = if self.revoked.contains(credential_id) {
            let pos = self.revoked.iter().position(|id| id == credential_id)?;
            RevocationWitness::Revoked {
                proof: disclosure_proof(&self.revocation_leaves(), pos)?,
            }
        } else {
            let leaves = self.revocation_leaves();
            let ids: Vec<&String> = self.revoked.iter().collect();
            let lower = ids
                .iter()
                .rposition(|id| id.as_str() < credential_id)
                .and_then(|i| Some((ids[i].clone(), disclosure_proof(&leaves, i)?)));
            let upper = ids
                .iter()
                .position(|id| id.as_str() > credential_id)
                .and_then(|i| Some((ids[i].clone(), disclosure_proof(&leaves, i)?)));
            RevocationWitness::Clear {
                leaf_count: ids.len(),
                lower,
                upper,
            }
        };
        Some(IdentityWitness {
            record: RecordWitness::from_record(subject, record),
            record_path,
            credential_id: credential_id.to_string(),
            revocation,
        })
    }
}

/// WIRING: consumed by the cross-domain verifier slice; the tests here
/// are the interim consumers.
/// Checks a witness against an anchor without consulting any registry.
///
/// The order of checks is the order of trust: shape first (a malformed path
/// is refused before a single hash is believed), then the revocation verdict
/// (a revoked credential is REFUSED even though its proof verified - that is
/// what the proof was for), and only then the anchor equation, which is what
/// binds the whole answer to a finalised block.
///
/// # Errors
///
/// `WitnessError::Malformed` for a witness that is not a well-formed claim,
/// `WitnessError::Revoked` for a verified revocation, `WitnessError::
/// RootMismatch` when the walked roots do not hash to `anchor`.
pub fn verify_identity_witness(
    anchor: &[u8; 32],
    witness: &IdentityWitness,
) -> Result<(), WitnessError> {
    let root1 = walk_to_root(witness.record.leaf_digest(), &witness.record_path)
        .ok_or(WitnessError::Malformed)?;
    let root2 = match &witness.revocation {
        RevocationWitness::Revoked { proof } => {
            // The revocation proof must itself verify before the refusal is
            // issued: "revoked" as a verdict has to be as trustworthy as
            // "clear", and a caller must not be able to burn a credential
            // with a garbage path.
            let _ = walk_to_root(revocation_leaf(&witness.credential_id), proof)
                .ok_or(WitnessError::Malformed)?;
            return Err(WitnessError::Revoked);
        }
        RevocationWitness::Clear {
            leaf_count,
            lower,
            upper,
        } => match (lower, upper) {
            (None, None) if *leaf_count == 0 => [0u8; 32],
            (None, None) => return Err(WitnessError::Malformed),
            (Some((lid, lp)), None) => {
                // Mirror of the upper-only arm: q is claimed ABOVE the
                // maximum, so the lower neighbour must be the tree's LAST
                // leaf (index leaf_count-1).
                if lp.leaf_index + 1 != *leaf_count {
                    return Err(WitnessError::Malformed);
                }
                if !(lid.as_str() < witness.credential_id.as_str()) {
                    return Err(WitnessError::Malformed);
                }
                walk_to_root(revocation_leaf(lid), lp).ok_or(WitnessError::Malformed)?
            }
            (None, Some((uid, up))) => {
                // "no lower neighbour" is a claim about position: the upper
                // must be the tree's FIRST leaf (index 0), or the gap sits
                // above some unmentioned smaller id - possibly q itself,
                // revoked, sitting before a non-first upper.
                if up.leaf_index != 0 {
                    return Err(WitnessError::Malformed);
                }
                if !(witness.credential_id.as_str() < uid.as_str()) {
                    return Err(WitnessError::Malformed);
                }
                walk_to_root(revocation_leaf(uid), up).ok_or(WitnessError::Malformed)?
            }
            (Some((lid, lp)), Some((uid, up))) => {
                // The claimed q must sit BETWEEN the two neighbours. The
                // adjacency check below proves the neighbours are next to
                // each other; this check proves the pair straddles q. Either
                // half alone proves nothing: an unrelated adjacent pair,
                // walked honestly to the true root, would otherwise read as
                // "q is in the gap" for EVERY revoked q. The comparison is
                // byte-order on the hex string, which is the set's own
                // ordering (BTreeSet<String>).
                if !(lid.as_str() < witness.credential_id.as_str()
                    && witness.credential_id.as_str() < uid.as_str())
                {
                    return Err(WitnessError::Malformed);
                }
                let l = walk_to_root(revocation_leaf(lid), lp).ok_or(WitnessError::Malformed)?;
                let u = walk_to_root(revocation_leaf(uid), up).ok_or(WitnessError::Malformed)?;
                // Adjacency first: two paths to the same root from
                // non-adjacent positions say nothing about the gap.
                if up.leaf_index != lp.leaf_index + 1 {
                    return Err(WitnessError::Malformed);
                }
                if l != u {
                    return Err(WitnessError::Malformed);
                }
                l
            }
        },
    };
    if &identity_anchor(&root1, &root2) != anchor {
        return Err(WitnessError::RootMismatch);
    }
    Ok(())
}

/// Why a cross-domain identity claim was refused even though the responder
/// answered politely. The three answers are deliberately distinct: a caller
/// that lumps them cannot tell "the registry says revoked" from "you handed
/// me bytes that never entered any tree" - the first is a verdict, the
/// second is the responder lying.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimError {
    /// The witness itself: `RootMismatch` (stale or foreign anchor),
    /// `Revoked` (a verified revocation) or `Malformed` (not a claim).
    Witness(WitnessError),
    /// The presented credential does not hash to the `credential_root`
    /// the witness committed to: the proof describes a different
    /// credential than the bytes being carried alongside it.
    RootBinding,
    /// The presented credential belongs to another subject than the one
    /// being claimed, or than the witness's record: a claim is bound to
    /// its subject on both legs, so no bystander can adopt a neighbor's
    /// honest proof as their own.
    SubjectBinding,
}

/// The cross-domain acceptance decision: one credential, one subject, one
/// FINALISED registry anchor - and no trust in whoever served the witness.
///
/// The sentence KIMLIK-MIMARI promised - "commitment + Merkle proof al,
/// finalize edilmiş PoA kökü karşısında doğrula" - is exactly this
/// function's three moves: the witness proves membership of the subject's
/// record AND non-membership of the credential id in the revocation tree at
/// `anchor`; the credential's own field-Merkle root must equal the
/// `credential_root` the witness committed to (otherwise the honest proof
/// is about a different credential and these bytes ride outside it); and
/// the credential's subject must be the witness's subject. Calendar
/// validity (issued/expired windows, the six gates of `is_credential_valid`)
/// stays the node-side question the anchor cannot answer: the root proves
/// what the registry recorded, not what time it is.
///
/// # Errors
///
/// `ClaimError::Witness` for any witness refusal (including a verified
/// revocation), then the two binding refusals, in that order.
#[must_use]
pub fn verify_identity_claim(
    anchor: &[u8; 32],
    subject: &Address,
    credential: &CredentialCommitment,
    witness: &IdentityWitness,
) -> Result<(), ClaimError> {
    verify_identity_witness(anchor, witness).map_err(ClaimError::Witness)?;
    if credential.subject.as_bytes() != subject.as_bytes() {
        return Err(ClaimError::SubjectBinding);
    }
    if credential.subject.as_bytes() != &witness.record.subject {
        return Err(ClaimError::SubjectBinding);
    }
    if credential.root() != witness.record.credential_root {
        return Err(ClaimError::RootBinding);
    }
    Ok(())
}

#[cfg(test)]
mod registry_witness_tests {
    use super::*;
    use crate::core::address::Address;

    fn addr(byte: u8) -> Address {
        Address([byte; 32])
    }

    fn method(byte: u8) -> VerificationMethod {
        VerificationMethod::new([byte; 32], MethodKind::MlDsa87)
    }

    fn registered(subject: u8) -> IdentityRecord {
        IdentityRecord::new(addr(subject), vec![method(subject)], vec![], 0).expect("valid record")
    }

    fn credential_for(subject: u8, value: u8) -> CredentialCommitment {
        let salt = [value; 32];
        CredentialCommitment {
            issuer: addr(9),
            subject: addr(subject),
            schema: "kycc-lite-v1".to_string(),
            fields: vec![FieldCommitment {
                name: "legal_name".to_string(),
                commitment: field_commitment(
                    "kycc-lite-v1",
                    "legal_name",
                    &salt,
                    &hash_fields_bytes(&[b"v", &[value]]),
                ),
            }],
            issued_at: 100,
            expires_at: Some(1_000),
        }
    }

    fn registry_with(subjects: &[u8]) -> IdentityRegistry {
        // `issue` refuses an issuer that is neither a registered DID nor a
        // live registered key of the subject (the credential-farm guard) -
        // `credential_for` names addr(9) as issuer, so the issuer record is
        // part of every registry these tests build.
        if !subjects.contains(&9) {
            let mut with_issuer = vec![9u8];
            with_issuer.extend_from_slice(subjects);
            return registry_with_sorted(&with_issuer);
        }
        registry_with_sorted(subjects)
    }

    fn registry_with_sorted(subjects: &[u8]) -> IdentityRegistry {
        let poa = crate::domain::ConsensusKind::PoA;
        let mut registry = IdentityRegistry::new();
        for s in subjects {
            registry
                .apply(
                    &poa,
                    IdentityOp::Register {
                        record: registered(*s),
                    },
                    100,
                )
                .expect("registration");
        }
        registry
    }

    fn issued(registry: &mut IdentityRegistry, subject: u8, value: u8) -> String {
        let poa = crate::domain::ConsensusKind::PoA;
        let credential = credential_for(subject, value);
        let id = credential_id(&credential);
        registry
            .apply(&poa, IdentityOp::Issue { credential }, 100)
            .expect("issue");
        // The map key is the hex encoding the registry itself chose.
        registry
            .credentials
            .iter()
            .find(|(_, c)| credential_id(c) == id)
            .map(|(k, _)| k.clone())
            .expect("issued credential is keyed")
    }

    fn revoked(registry: &mut IdentityRegistry, id: &str, subject: u8, value: u8) {
        let poa = crate::domain::ConsensusKind::PoA;
        let credential = credential_for(subject, value);
        registry
            .apply(&poa, IdentityOp::Revoke { credential }, 200)
            .expect("revoke");
        assert!(
            registry.revoked.contains(&id.to_string()),
            "the id landed in the set"
        );
    }

    #[test]
    fn the_empty_registry_root_is_the_anchor_of_two_empty_trees() {
        let registry = IdentityRegistry::new();
        let zero = [0u8; 32];
        assert_eq!(registry.root(), identity_anchor(&zero, &zero));
        // and is NOT the retired accumulator seed - if someone "fixes"
        // root() back to the single fold, this pins which value is law.
        assert_ne!(
            registry.root(),
            hash_fields_bytes(&[b"bud-identity-root-v1"])
        );
    }

    #[test]
    fn a_witness_verifies_against_the_live_anchor() {
        let mut registry = registry_with(&[1, 2, 3]);
        let id = issued(&mut registry, 2, 7);
        let witness = registry
            .witness_for(&addr(2), &id)
            .expect("subject 2 exists");
        let anchor = registry.root();
        assert_eq!(verify_identity_witness(&anchor, &witness), Ok(()));
    }

    #[test]
    fn a_stale_anchor_refuses_an_otherwise_perfect_witness() {
        let mut registry = registry_with(&[1, 2]);
        let id = issued(&mut registry, 2, 7);
        let witness = registry.witness_for(&addr(2), &id).expect("witness");
        let before = registry.root();
        // A third subject moves the tree. The witness stays honest at the
        // anchor it was captured at, and is refused the moment the anchor
        // moves - an anchor that accepted both would anchor nothing.
        registry
            .apply(
                &crate::domain::ConsensusKind::PoA,
                IdentityOp::Register {
                    record: registered(3),
                },
                150,
            )
            .expect("third registration");
        assert_eq!(verify_identity_witness(&before, &witness), Ok(()));
        assert_eq!(
            verify_identity_witness(&registry.root(), &witness),
            Err(WitnessError::RootMismatch)
        );
    }

    #[test]
    fn revocation_flips_the_verdict_through_the_same_anchor_pair() {
        let mut registry = registry_with(&[1, 2]);
        let id = issued(&mut registry, 2, 7);
        let anchor_before = registry.root();
        assert_eq!(
            verify_identity_witness(
                &anchor_before,
                &registry.witness_for(&addr(2), &id).expect("witness")
            ),
            Ok(())
        );
        revoked(&mut registry, &id, 2, 7);
        let anchor_after = registry.root();
        assert_ne!(
            anchor_before, anchor_after,
            "the revocation tree moved the anchor"
        );
        assert_eq!(
            verify_identity_witness(
                &anchor_after,
                &registry.witness_for(&addr(2), &id).expect("witness")
            ),
            Err(WitnessError::Revoked)
        );
        // A DIFFERENT credential of the same subject is untouched: the
        // revocation is per id, not per subject.
        let other = issued(&mut registry, 2, 8);
        assert_eq!(
            verify_identity_witness(
                &registry.root(),
                &registry.witness_for(&addr(2), &other).expect("witness")
            ),
            Ok(())
        );
    }

    #[test]
    fn exclusion_uses_the_two_neighbours_and_refuses_a_fabricated_gap() {
        let mut registry = registry_with(&[1]);
        let a = issued(&mut registry, 1, 1);
        let b = issued(&mut registry, 1, 2);
        let c = issued(&mut registry, 1, 3);
        revoked(&mut registry, &a, 1, 1);
        revoked(&mut registry, &b, 1, 2);
        // c sits above the maximum revoked id: one-sided exclusion, and it
        // must verify...
        let witness_c = registry.witness_for(&addr(1), &c).expect("witness");
        assert_eq!(
            verify_identity_witness(&registry.root(), &witness_c),
            Ok(())
        );
        // ...but a claim of "the revocation tree is empty" over a two-leaf
        // tree is not a claim at all: with count=2, at least one side must
        // carry a neighbour, and the verifier refuses the shape before it
        // believes any digest. (Pointing a fabricated neighbour at the true
        // root is infeasible without a hash collision - which is why the
        // shape check, not a forged-path check, is the reachable attack.)
        let fake = IdentityWitness {
            credential_id: "0".repeat(64),
            revocation: RevocationWitness::Clear {
                leaf_count: 2,
                lower: None,
                upper: None,
            },
            ..witness_c.clone()
        };
        assert_eq!(
            verify_identity_witness(&registry.root(), &fake),
            Err(WitnessError::Malformed),
            "a claimed empty exclusion over a two-leaf tree is not a claim"
        );
        // The upper-only arm must be anchored at index 0: c is above the
        // max, so its honest witness is lower-only (last leaf). Re-spelling
        // the same proof as "upper b sits at index 1 above q" is refused -
        // position, not just ordering, is the claim.
        let clear = match &witness_c.revocation {
            RevocationWitness::Clear { lower, .. } => lower.clone().expect("lower neighbour"),
            _ => panic!("c sits above both revocations"),
        };
        let shifted = IdentityWitness {
            credential_id: "ff".repeat(32),
            revocation: RevocationWitness::Clear {
                leaf_count: 2,
                lower: None,
                upper: Some(clear.clone()),
            },
            ..witness_c.clone()
        };
        assert_eq!(
            verify_identity_witness(&registry.root(), &shifted),
            Err(WitnessError::Malformed),
            "an upper-only claim whose upper is not the first leaf is a fabricated gap"
        );
        // Honest lower-only: the true witness for c keeps its own lower
        // (max leaf) and only changes q to something above it.
        let honest_above = IdentityWitness {
            credential_id: "ff".repeat(32),
            revocation: RevocationWitness::Clear {
                leaf_count: 2,
                lower: Some(clear),
                upper: None,
            },
            ..witness_c
        };
        assert_eq!(
            verify_identity_witness(&registry.root(), &honest_above),
            Ok(())
        );
    }

    #[test]
    fn insertion_order_is_not_part_of_the_root() {
        let one = registry_with(&[1, 2, 3]);
        let other = registry_with(&[3, 1, 2]);
        assert_eq!(
            one.root(),
            other.root(),
            "leaves are ordered by subject, not by history"
        );
        // A witness built on one registry verifies against the other's
        // anchor: the trees are the same tree.
        let witness = one
            .witness_for(&addr(2), "nonexistent-id")
            .expect("record witness");
        let mut witness = witness;
        witness.credential_id = "ff".repeat(32);
        if let RevocationWitness::Clear {
            leaf_count,
            lower,
            upper,
        } = &mut witness.revocation
        {
            assert_eq!(
                (*leaf_count, lower.is_some(), upper.is_some()),
                (0, false, false)
            );
        }
        assert_eq!(verify_identity_witness(&other.root(), &witness), Ok(()));
    }

    #[test]
    fn the_leaf_formula_is_pinned_to_the_witness_path() {
        // root() must be recomputable from the public parts alone - if
        // either fold drifts while the other is updated, this fails.
        let registry = registry_with(&[4]);
        // registry_with registers the issuer (addr 9) as well; records sort
        // by subject, and [4, 9] is that order.
        let leaf4 = RecordWitness::from_record(&addr(4), &registered(4)).leaf_digest();
        let leaf9 = RecordWitness::from_record(&addr(9), &registered(9)).leaf_digest();
        let root1 = merkle_root(&[leaf4, leaf9]);
        let zero = [0u8; 32];
        assert_eq!(registry.root(), identity_anchor(&root1, &zero));
    }

    #[test]
    fn tampering_with_the_witness_data_breaks_the_digest_not_the_walk() {
        let mut registry = registry_with(&[1, 2]);
        let id = issued(&mut registry, 2, 7);
        let anchor = registry.root();
        let mut witness = registry.witness_for(&addr(2), &id).expect("witness");
        // A bumped guardian list changes the leaf; the path walks a
        // different digest to a different root, so the anchor equation
        // fails - the refusal is RootMismatch, never a panic and never Ok.
        witness.record.guardians.push([9u8; 32]);
        assert_eq!(
            verify_identity_witness(&anchor, &witness),
            Err(WitnessError::RootMismatch)
        );
    }
    #[test]
    fn a_claim_binds_credential_bytes_to_the_anchor_through_the_witness() {
        let poa = crate::domain::ConsensusKind::PoA;
        let mut registry = registry_with(&[2]);
        let credential = credential_for(2, 7);
        registry
            .apply(
                &poa,
                IdentityOp::Issue {
                    credential: credential.clone(),
                },
                100,
            )
            .expect("issue");
        let id = registry
            .credentials
            .iter()
            .find(|(_, c)| credential_id(c) == credential_id(&credential))
            .map(|(k, _)| k.clone())
            .expect("keyed");
        let witness = registry.witness_for(&addr(2), &id).expect("witness");
        let anchor = registry.root();
        assert_eq!(
            verify_identity_claim(&anchor, &addr(2), &credential, &witness),
            Ok(())
        );
        // The same witness, a DIFFERENT credential of the same subject:
        // the bytes must not be able to ride the neighboring proof.
        let other = credential_for(2, 8);
        assert_eq!(
            verify_identity_claim(&anchor, &addr(2), &other, &witness),
            Err(ClaimError::RootBinding)
        );
        // The right bytes, the wrong subject claimed: refused on binding.
        assert_eq!(
            verify_identity_claim(&anchor, &addr(3), &credential, &witness),
            Err(ClaimError::SubjectBinding)
        );
        // A revocation turns the same call into a verdict, not an error of
        // shape: Witness(Revoked) is the registry SAYING NO through honest
        // proofs.
        registry
            .apply(
                &poa,
                IdentityOp::Revoke {
                    credential: credential.clone(),
                },
                200,
            )
            .expect("revoke");
        let revoked_witness = registry.witness_for(&addr(2), &id).expect("witness");
        assert_eq!(
            verify_identity_claim(&registry.root(), &addr(2), &credential, &revoked_witness),
            Err(ClaimError::Witness(WitnessError::Revoked))
        );
        // ...and the pre-revocation witness against the POST-revocation
        // anchor no longer even speaks: RootMismatch, the stale answer dies.
        assert_eq!(
            verify_identity_claim(&registry.root(), &addr(2), &credential, &witness),
            Err(ClaimError::Witness(WitnessError::RootMismatch))
        );
    }
}
