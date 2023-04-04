use super::db::SqliteStorage;
use crate::boltzswap::BoltzApiReverseSwapStatus;
use crate::{ReverseSwapInfo, ReverseSwapStatus};
use anyhow::Result;
use rusqlite::{named_params, Row};

impl SqliteStorage {
    pub(crate) fn insert_reverse_swap(&self, rsi: &ReverseSwapInfo) -> Result<()> {
        self.get_connection()?.execute(
            "INSERT INTO reverse_swaps (id, created_at, local_preimage, local_private_key, destination_address, boltz_api_status, lockup_address, hodl_bolt11, onchain_amount_sat, redeem_script)\
            VALUES (:id, :created_at, :local_preimage, :local_private_key, :destination_address, :boltz_api_status, :lockup_address, :hodl_bolt11, :onchain_amount_sat, :redeem_script)",
            named_params! {
                ":id": rsi.id,
                ":created_at": rsi.created_at,
                ":local_preimage": rsi.local_preimage,
                ":local_private_key": rsi.local_private_key,
                ":destination_address": rsi.destination_address,
                ":boltz_api_status": rsi.boltz_api_status,
                ":lockup_address": rsi.lockup_address,
                ":hodl_bolt11": rsi.hodl_bolt11,
                ":onchain_amount_sat": rsi.onchain_amount_sat,
                ":redeem_script": rsi.redeem_script
            },
        )?;

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
        let mut stmt = con.prepare(
            "
           SELECT * FROM reverse_swaps
         ",
        )?;

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
            id: row.get(0)?,
            created_at: row.get(1)?,
            local_preimage: row.get(2)?,
            local_private_key: row.get(3)?,
            destination_address: row.get(4)?,
            boltz_api_status: row.get(5)?,
            lockup_address: row.get(6)?,
            hodl_bolt11: row.get(7)?,
            onchain_amount_sat: row.get(8)?,
            redeem_script: row.get(9)?,
        })
    }
}
