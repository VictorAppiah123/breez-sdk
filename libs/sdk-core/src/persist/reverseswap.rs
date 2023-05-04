use super::db::SqliteStorage;
use crate::boltzswap::BoltzApiReverseSwapStatus;
use crate::{ReverseSwapInfo, ReverseSwapInfoCached, ReverseSwapStatus};
use anyhow::Result;
use rusqlite::{named_params, Row};

impl SqliteStorage {
    pub(crate) fn insert_reverse_swap(&self, rsi: &ReverseSwapInfo) -> Result<()> {
        let mut con = self.get_connection()?;
        let tx = con.transaction()?;

        tx.execute(
            "INSERT INTO reverse_swaps (id, created_at, local_preimage, local_private_key, destination_address, boltz_api_status, hodl_bolt11, redeem_script)\
            VALUES (:id, :created_at, :local_preimage, :local_private_key, :destination_address, :boltz_api_status, :hodl_bolt11, :redeem_script)",
            named_params! {
                ":id": rsi.id,
                ":created_at": rsi.created_at,
                ":local_preimage": rsi.local_preimage,
                ":local_private_key": rsi.local_private_key,
                ":destination_address": rsi.destination_address,
                ":boltz_api_status": rsi.boltz_api_status,
                ":hodl_bolt11": rsi.hodl_bolt11,
                ":redeem_script": rsi.redeem_script
            },
        )?;

        tx.execute(
            "INSERT INTO reverse_swaps_info (id, lockup_address, onchain_amount_sat)\
            VALUES (:id, :lockup_address, :onchain_amount_sat)",
            named_params! {
                ":id": rsi.id,
                ":lockup_address": rsi.cache.lockup_address,
                ":onchain_amount_sat": rsi.cache.onchain_amount_sat
            },
        )?;

        tx.commit()?;
        Ok(())
    }

    pub(crate) fn update_reverse_swap_boltz_status(
        &self,
        id: &str,
        boltz_status: &BoltzApiReverseSwapStatus,
    ) -> Result<()> {
        self.get_connection()?.execute(
            "UPDATE reverse_swaps SET boltz_api_status=:boltz_api_status where id=:id",
            named_params! {
             ":boltz_api_status": boltz_status,
             ":id": id,
            },
        )?;

        Ok(())
    }

    pub(crate) fn list_reverse_swaps(&self) -> Result<Vec<ReverseSwapInfo>> {
        let con = self.get_connection()?;
        let mut stmt = con.prepare(&self.select_reverse_swap_query())?;

        let vec: Vec<ReverseSwapInfo> = stmt
            .query_map([], |row| self.sql_row_to_reverse_swap(row))?
            .map(|i| i.unwrap())
            .collect();

        Ok(vec)
    }

    /// Returns the reverse swaps for which we expect the status to change, and therefore need
    /// to be monitored.
    pub(crate) fn get_monitored_reverse_swaps(&self) -> Result<Vec<ReverseSwapInfo>> {
        // Exclude "final" statuses, e.g. from which the swap cannot transition
        let non_monitored_states = vec![ReverseSwapStatus::Expired, ReverseSwapStatus::ClaimTxSeen];
        let matching_reverse_swaps: Vec<ReverseSwapInfo> = self
            .list_reverse_swaps()?
            .iter()
            .filter(|&rs| !non_monitored_states.contains(&rs.status()))
            .cloned()
            .collect();
        Ok(matching_reverse_swaps)
    }

    fn sql_row_to_reverse_swap(&self, row: &Row) -> Result<ReverseSwapInfo, rusqlite::Error> {
        Ok(ReverseSwapInfo {
            id: row.get("id")?,
            created_at: row.get("created_at")?,
            local_preimage: row.get("local_preimage")?,
            local_private_key: row.get("local_private_key")?,
            destination_address: row.get("destination_address")?,
            boltz_api_status: row.get("boltz_api_status")?,
            hodl_bolt11: row.get("hodl_bolt11")?,
            redeem_script: row.get("redeem_script")?,
            cache: ReverseSwapInfoCached {
                lockup_address: row.get("lockup_address")?,
                onchain_amount_sat: row.get("onchain_amount_sat")?,
            },
        })
    }

    fn select_reverse_swap_query(&self) -> String {
        "
            SELECT
             *
            FROM reverse_swaps
             LEFT JOIN reverse_swaps_info ON reverse_swaps.id = reverse_swaps_info.id
            "
        .to_string()
    }
}
