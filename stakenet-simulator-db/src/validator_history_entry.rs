use crate::{big_decimal_u64::BigDecimalU64, decode_db, error::StakenetSimulatorDbError};
use serde::{Deserialize, Serialize};
use sqlx::{
    Error, FromRow, Pool, Postgres, QueryBuilder, Row,
    postgres::PgRow,
    types::{BigDecimal, Json},
};
use validator_history::{
    ClientVersion as JitoClientVersion, ValidatorHistoryEntry as JitoValidatorHistoryEntry,
};

#[derive(Clone)]
pub struct ValidatorHistoryEntry {
    pub id: String,
    pub vote_pubkey: String,
    pub validator_history_entry: JitoValidatorHistoryEntry,
}

impl FromRow<'_, PgRow> for ValidatorHistoryEntry {
    fn from_row(row: &PgRow) -> Result<Self, Error> {
        let id = row.try_get("id")?;
        let vote_pubkey = row.try_get("vote_pubkey")?;
        let activated_stake_lamports: BigDecimalU64 = row.try_get("activated_stake_lamports")?;
        let epoch: i32 = row.try_get("epoch")?;
        let mev_commission: i32 = row.try_get("mev_commission")?;
        let epoch_credits: i64 = row.try_get("epoch_credits")?;
        let commission: i32 = row.try_get("commission")?;
        let client_type: i16 = row.try_get("client_type")?;
        let version: ClientVersion =
            serde_json::from_value(row.try_get("version")?).map_err(|_| {
                Error::Decode(Box::new(StakenetSimulatorDbError::DecodeError(
                    String::from("version"),
                )))
            })?;
        let ip: String = row.try_get("ip")?;
        let merkle_root_upload_authority: i16 = row.try_get("merkle_root_upload_authority")?;
        let is_superminority: i16 = row.try_get("is_superminority")?;
        let rank: i64 = row.try_get("rank")?;
        let vote_account_last_update_slot: BigDecimalU64 =
            row.try_get("vote_account_last_update_slot")?;
        let mev_earned: i64 = row.try_get("mev_earned")?;
        let priority_fee_commission: i32 = row.try_get("priority_fee_commission")?;
        let priority_fee_tips: BigDecimalU64 = row.try_get("priority_fee_tips")?;
        let total_priority_fees: BigDecimalU64 = row.try_get("total_priority_fees")?;
        let total_leader_slots: i64 = row.try_get("total_leader_slots")?;
        let blocks_produced: i64 = row.try_get("blocks_produced")?;
        let block_data_updated_at_slot: BigDecimalU64 =
            row.try_get("block_data_updated_at_slot")?;
        let priority_fee_merkle_root_upload_authority: i16 =
            row.try_get("priority_fee_merkle_root_upload_authority")?;

        let ip_converted: [u8; 4] = ip
            .split(".")
            .map(|x| x.parse::<u8>().unwrap())
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap();

        Ok(Self {
            id,
            vote_pubkey,
            validator_history_entry: JitoValidatorHistoryEntry {
                activated_stake_lamports: activated_stake_lamports.into(),
                epoch: decode_db!(epoch, "epoch"),
                mev_commission: decode_db!(mev_commission, "mev_commission"),
                epoch_credits: decode_db!(epoch_credits, "epoch_credits"),
                commission: decode_db!(commission, "commission"),
                client_type: decode_db!(client_type, "client_type"),
                version: JitoClientVersion {
                    major: version.major,
                    minor: version.minor,
                    patch: version.patch,
                },
                ip: ip_converted,
                merkle_root_upload_authority: int_to_upload_authority(merkle_root_upload_authority),
                is_superminority: decode_db!(is_superminority, "is_superminority"),
                rank: decode_db!(rank, "rank"),
                vote_account_last_update_slot: vote_account_last_update_slot.into(),
                mev_earned: decode_db!(mev_earned, "mev_earned"),
                priority_fee_commission: decode_db!(
                    priority_fee_commission,
                    "priority_fee_commission"
                ),
                priority_fee_tips: priority_fee_tips.into(),
                total_priority_fees: total_priority_fees.into(),
                total_leader_slots: decode_db!(total_leader_slots, "total_leader_slots"),
                blocks_produced: decode_db!(blocks_produced, "blocks_produced"),
                block_data_updated_at_slot: block_data_updated_at_slot.into(),
                priority_fee_merkle_root_upload_authority: int_to_upload_authority(
                    priority_fee_merkle_root_upload_authority,
                ),
                ..JitoValidatorHistoryEntry::default()
            },
        })
    }
}

fn int_to_upload_authority(int: i16) -> validator_history::MerkleRootUploadAuthority {
    match int {
        0 | 255 => validator_history::MerkleRootUploadAuthority::Unset,
        1 => validator_history::MerkleRootUploadAuthority::Other,
        2 => validator_history::MerkleRootUploadAuthority::OldJitoLabs,
        3 => validator_history::MerkleRootUploadAuthority::TipRouter,
        4 => validator_history::MerkleRootUploadAuthority::DNE,
        a => {
            panic!("unknown {}", a)
        }
    }
}

impl ValidatorHistoryEntry {
    /// Number of fields in the validator_history_entries table
    const NUM_FIELDS: u8 = 22;
    // Based on the bind limit of postgres
    const INSERT_CHUNK_SIZE: usize = 65534 / Self::NUM_FIELDS as usize;
    const INSERT_QUERY: &str = "INSERT INTO validator_history_entries (id,vote_pubkey,activated_stake_lamports,epoch,mev_commission,epoch_credits,commission,client_type,version,ip,merkle_root_upload_authority,is_superminority,rank,vote_account_last_update_slot,mev_earned,priority_fee_commission,priority_fee_tips,total_priority_fees,total_leader_slots,blocks_produced,block_data_updated_at_slot,priority_fee_merkle_root_upload_authority) VALUES ";

    pub fn new(vote_pubkey: String, validator_history_entry: JitoValidatorHistoryEntry) -> Self {
        Self {
            id: format!("{}-{}", validator_history_entry.epoch, vote_pubkey),
            vote_pubkey,
            validator_history_entry,
        }
    }

    pub async fn bulk_insert(
        db_connection: &Pool<Postgres>,
        records: Vec<Self>,
    ) -> Result<(), Error> {
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
            separated.push_bind(record.id);
            separated.push_bind(record.vote_pubkey);
            separated.push_bind(BigDecimal::from(
                record.validator_history_entry.activated_stake_lamports,
            ));
            separated.push_bind(i32::from(record.validator_history_entry.epoch));
            separated.push_bind(i32::from(record.validator_history_entry.mev_commission));
            separated.push_bind(i64::from(record.validator_history_entry.epoch_credits));
            separated.push_bind(i32::from(record.validator_history_entry.commission));
            separated.push_bind(i16::from(record.validator_history_entry.client_type));
            let version: ClientVersion = record.validator_history_entry.version.into();
            separated.push_bind(Json(version));
            separated.push_bind(
                record
                    .validator_history_entry
                    .ip
                    .into_iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>()
                    .join("."),
            );

            separated.push_bind(i16::from(
                record.validator_history_entry.merkle_root_upload_authority as u8,
            ));
            separated.push_bind(i16::from(record.validator_history_entry.is_superminority));
            separated.push_bind(i64::from(record.validator_history_entry.rank));
            separated.push_bind(BigDecimal::from(
                record.validator_history_entry.vote_account_last_update_slot,
            ));
            separated.push_bind(BigDecimal::from(record.validator_history_entry.mev_earned));

            // This is done as these fields were not tracked by the validator history porgram before this epoch
            let priority_fee_commission = if record.validator_history_entry.epoch <= 735 {
                0
            } else {
                i32::from(record.validator_history_entry.priority_fee_commission)
            };
            separated.push_bind(priority_fee_commission);

            let priority_fee_tips = if record.validator_history_entry.epoch <= 735 {
                BigDecimal::from(0)
            } else {
                BigDecimal::from(record.validator_history_entry.priority_fee_tips)
            };
            separated.push_bind(priority_fee_tips);

            separated.push_bind(BigDecimal::from(
                record.validator_history_entry.total_priority_fees,
            ));
            separated.push_bind(i64::from(record.validator_history_entry.total_leader_slots));
            separated.push_bind(i64::from(record.validator_history_entry.blocks_produced));
            separated.push_bind(BigDecimal::from(
                record.validator_history_entry.block_data_updated_at_slot,
            ));

            // TODO: The simulator is looking at historical data that has already been set to DNE. Hence,
            // for now to prevent a lot of DNE's we currently modify the data to change all the DNE's to UNSET
            // To be reverted when this issue is fixed
            let mut value = record
                .validator_history_entry
                .priority_fee_merkle_root_upload_authority as u8;

            if value == 4 {
                value = 255;
            }

            separated.push_bind(i16::from(value));

            separated.push_unseparated(") ");

            if num_records >= Self::INSERT_CHUNK_SIZE {
                query_builder.push(" ON CONFLICT (id) DO NOTHING");
                let query = query_builder.build();
                query.execute(db_connection).await?;
                num_records = 0;
                query_builder = QueryBuilder::new(Self::INSERT_QUERY);
            }
        }

        if num_records > 0 {
            query_builder.push(" ON CONFLICT (id) DO NOTHING");
            let query = query_builder.build();
            query.execute(db_connection).await?;
        }
        Ok(())
    }

    pub async fn fetch_by_validator(
        db_connection: &Pool<Postgres>,
        vote_pubkey: &str,
    ) -> Result<Vec<Self>, Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM validator_history_entries WHERE vote_pubkey = $1")
            .bind(vote_pubkey)
            .fetch_all(db_connection)
            .await
    }

    pub async fn fetch_by_validator_and_epoch(
        db_connection: &Pool<Postgres>,
        vote_pubkey: &str,
        epoch: u64,
    ) -> Result<Option<Self>, Error> {
        let id = format!("{}-{}", epoch, vote_pubkey);
        sqlx::query_as::<_, Self>("SELECT * FROM validator_history_entries WHERE id = $1")
            .bind(id)
            .fetch_optional(db_connection)
            .await
    }

    pub async fn fetch_all_records_between_epochs(
        db_connection: &Pool<Postgres>,
        start_epoch: u64,
        end_epoch: u64,
    ) -> Result<Vec<Self>, Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM validator_history_entries WHERE epoch >= $1 AND epoch <= $2",
        )
        .bind(start_epoch as i32)
        .bind(end_epoch as i32)
        .fetch_all(db_connection)
        .await
    }

    pub async fn get_all_vote_pubkeys(
        db_connection: &Pool<Postgres>,
    ) -> Result<Vec<String>, Error> {
        let pubkeys = sqlx::query_as::<_, VotePubkey>("SELECT DISTINCT ON(vote_pubkey) vote_pubkey FROM validator_history_entries GROUP BY vote_pubkey")
        .fetch_all(db_connection)
        .await?;

        Ok(pubkeys.into_iter().map(|row| row.vote_pubkey).collect())
    }

    pub async fn fetch_all_validator_history_entries(
        db_connection: &Pool<Postgres>,
    ) -> Result<Vec<Self>, Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM validator_history_entries")
            .fetch_all(db_connection)
            .await
    }
}

#[derive(FromRow)]
struct VotePubkey {
    vote_pubkey: String,
}

#[derive(Deserialize, Serialize)]
pub struct ClientVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u16,
}
impl From<JitoClientVersion> for ClientVersion {
    fn from(value: JitoClientVersion) -> Self {
        Self {
            major: value.major,
            minor: value.minor,
            patch: value.patch,
        }
    }
}
