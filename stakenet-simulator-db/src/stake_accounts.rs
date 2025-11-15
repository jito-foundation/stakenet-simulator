use solana_sdk::{pubkey::Pubkey, stake::state::StakeStateV2};
use sqlx::{Error as SqlxError, FromRow, Pool, Postgres, QueryBuilder, types::BigDecimal};

use crate::stake_accounts;

#[derive(Default)]
pub struct StakeAccount {
    pubkey: String,
    discriminator: u32,
    rent_exempt_reserve: Option<u64>,
    authorized_staker: Option<String>,
    authorized_withdrawer: Option<String>,
    lockup_unix_timestamp: Option<i64>,
    lockup_epoch: Option<u64>,
    lockup_custodian: Option<String>,
    /// Validator pubkey that the stake account is delegated to
    delegation_voter_pubkey: Option<String>,
    /// Amount of stake delegated to the validator
    delegation_stake: Option<u64>,
    /// Epoch when the stake account was activated
    delegation_activation_epoch: Option<u64>,
    /// Epoch when the stake account was deactivated
    delegation_deactivation_epoch: Option<u64>,
    delegation_warmup_cooldown_rate: Option<f64>,
    credits_observed: Option<u64>,
}

impl From<(Pubkey, StakeStateV2)> for StakeAccount {
    fn from(value: (Pubkey, StakeStateV2)) -> Self {
        let mut res = stake_accounts::StakeAccount {
            pubkey: value.0.to_string(),
            ..Self::default()
        };
        match value.1 {
            StakeStateV2::Uninitialized => {
                res.discriminator = 0;
                todo!("handle this state if needed in the future");
            }
            StakeStateV2::Initialized(_meta) => {
                res.discriminator = 1;
                todo!("handle this state if needed in the future");
            }
            StakeStateV2::Stake(meta, stake, _stake_flags) => {
                res.discriminator = 2;
                res.rent_exempt_reserve = Some(meta.rent_exempt_reserve);
                res.authorized_staker = Some(meta.authorized.staker.to_string());
                res.authorized_withdrawer = Some(meta.authorized.withdrawer.to_string());
                res.lockup_unix_timestamp = Some(meta.lockup.unix_timestamp);
                res.lockup_epoch = Some(meta.lockup.epoch);
                res.lockup_custodian = Some(meta.lockup.custodian.to_string());
                res.delegation_voter_pubkey = Some(stake.delegation.voter_pubkey.to_string());
                res.delegation_stake = Some(stake.delegation.stake);
                res.delegation_activation_epoch = Some(stake.delegation.activation_epoch);
                res.delegation_deactivation_epoch = Some(stake.delegation.deactivation_epoch);
                // warmup_cooldown_rate field was deprecated - it's now a network-wide constant
                // rather than per-delegation. Set to None since we don't have epoch context.
                res.delegation_warmup_cooldown_rate = None;
                res.credits_observed = Some(stake.credits_observed);
            }
            StakeStateV2::RewardsPool => {
                res.discriminator = 3;
                todo!("handle this state if needed in the future");
            }
        };
        res
    }
}

impl StakeAccount {
    const NUM_FIELDS: u8 = 14;
    // Based on the bind limit of postgres
    const INSERT_CHUNK_SIZE: usize = 65534 / Self::NUM_FIELDS as usize;
    const INSERT_QUERY: &str = "INSERT INTO stake_accounts (pubkey,discriminator,rent_exempt_reserve,authorized_staker,authorized_withdrawer,lockup_unix_timestamp,lockup_epoch,lockup_custodian,delegation_voter_pubkey,delegation_stake,delegation_activation_epoch,delegation_deactivation_epoch,delegation_warmup_cooldown_rate,credits_observed) VALUES ";

    pub async fn bulk_insert(
        db_connection: &Pool<Postgres>,
        records: Vec<Self>,
    ) -> Result<(), SqlxError> {
        if records.is_empty() {
            return Ok(());
        }

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(Self::INSERT_QUERY);
        let mut num_records: usize = 0;

        for record in records.into_iter() {
            num_records += 1;
            if num_records > 1 {
                query_builder.push(", (");
            } else {
                query_builder.push("(");
            }
            let mut separated = query_builder.separated(", ");
            separated.push_bind(record.pubkey);
            separated.push_bind(i32::try_from(record.discriminator).unwrap());
            separated.push_bind(record.rent_exempt_reserve.map(BigDecimal::from));
            separated.push_bind(record.authorized_staker);
            separated.push_bind(record.authorized_withdrawer);
            separated.push_bind(record.lockup_unix_timestamp);
            separated.push_bind(record.lockup_epoch.map(BigDecimal::from));
            separated.push_bind(record.lockup_custodian);
            separated.push_bind(record.delegation_voter_pubkey);
            separated.push_bind(record.delegation_stake.map(BigDecimal::from));
            separated.push_bind(record.delegation_activation_epoch.map(BigDecimal::from));
            separated.push_bind(record.delegation_deactivation_epoch.map(BigDecimal::from));
            separated.push_bind(record.delegation_warmup_cooldown_rate);
            separated.push_bind(record.credits_observed.map(BigDecimal::from));

            separated.push_unseparated(") ");

            if num_records >= Self::INSERT_CHUNK_SIZE {
                query_builder.push(" ON CONFLICT (pubkey) DO NOTHING");
                let query = query_builder.build();
                query.execute(db_connection).await?;
                num_records = 0;
                query_builder = QueryBuilder::new(Self::INSERT_QUERY);
            }
        }

        if num_records > 0 {
            query_builder.push(" ON CONFLICT (pubkey) DO NOTHING");
            let query = query_builder.build();
            query.execute(db_connection).await?;
        }
        Ok(())
    }

    pub async fn get_all_pubkeys(db_connection: &Pool<Postgres>) -> Result<Vec<String>, SqlxError> {
        let pubkeys =
            sqlx::query_as::<_, RecordPubkey>("SELECT pubkey FROM stake_accounts ORDER BY pubkey")
                .fetch_all(db_connection)
                .await?;

        Ok(pubkeys.into_iter().map(|row| row.pubkey).collect())
    }
}

#[derive(FromRow)]
struct RecordPubkey {
    pubkey: String,
}
