# Binance Global — Portfolio Margin Error Codes

**Exchange:** Binance Global  
**Source:** Error Codes doc (document index 22 in session)  
**Snapshot date:** 2026-07-02  
**Reviewed:** No

> Selected codes most relevant to TradeAutopsy circuit-breaker logic.

---

## General

| Code | Name | Message |
|---|---|---|
| `-1003` | TOO_MANY_REQUESTS | Rate limit exceeded |
| `-1007` | TIMEOUT | Response timeout — execution status unknown |
| `-1008` | Request Throttled | System-level protection. Reduce-only/close-position exempt |
| `-1021` | INVALID_TIMESTAMP | Outside recvWindow |
| `-1022` | INVALID_SIGNATURE | Signature mismatch |
| `-1125` | INVALID_LISTEN_KEY | Use `POST /papi/v1/listenKey` to recreate |
| `-2015` | REJECTED_MBX_KEY | Invalid key / IP / permissions |

## Order / Position

| Code | Message |
|---|---|
| `-2018` | Balance insufficient |
| `-2019` | Margin insufficient |
| `-2020` | Unable to fill |
| `-2021` | Order would immediately trigger |
| `-2022` | ReduceOnly rejected — conflicts with existing open orders |
| `-2023` | User in liquidation mode |
| `-2024` | Position insufficient |
| `-2025` | Max open order limit reached |
| `-2027` | Exceeded max allowable position at current leverage |
| `-2028` | Leverage too small — insufficient margin balance |

## Futures-specific

| Code | Message |
|---|---|
| `-4046` | No need to change margin type |
| `-4047` | Margin type cannot change with open orders |
| `-4048` | Margin type cannot change with open position |
| `-4060` | Invalid position side |
| `-4061` | Order's position side doesn't match user setting |
| `-4067` | Position side can't change with open orders |
| `-4068` | Position side can't change with open position |
| `-4220` | CM dual-side not allowed to differ from UM dual-side |
| `-4405` | Only reduceOnly order allowed |
| `-4407` | Restricted account: reduceOnly only on this symbol |
| `-4415` | CM reduceOnly only |
| `-4517` | Symbol under position risk control — reduceOnly only |
| `-5021` | FOK order rejected — could not fill immediately |
| `-5022` | GTX/Post Only rejected — would not be maker |
| `-5028` | Timestamp outside matching engine recvWindow |

## Margin-specific

| Code | Message |
|---|---|
| `-51006` | Exceeds maximum borrowable amount |
| `-51007` | Pending borrow or repayment — try again later |
| `-51014` | Asset not available for borrowing |
| `-51061` | Insufficient loanable assets due to high demand |
| `-51068` | Margin account in liquidation — cannot trade |
| `-51113` | Limit sell > 15% below index price / Limit buy > 15% above index price |
| `-51122` | Portfolio Margin account cannot use this function |
