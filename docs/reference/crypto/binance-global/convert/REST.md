# Binance Global — Convert REST API

**Exchange:** Binance Global  
**Source:** `schema__8_.yaml` (1,051 lines)  
**Snapshot date:** 2026-07-02  
**Reviewed:** No

Schema title: "Convert REST API — Request quotes and execute cryptocurrency conversions."

Rate limits guide: `products/convert/general-info#limits`  
Security guide: `products/convert/general-info#request-security`

---

## Endpoints (from `/sapi/v1/convert/`)

Core flow: get quote → accept quote → query status

```
GET    /sapi/v1/convert/exchangeInfo   Tradeable pairs and limits
POST   /sapi/v1/convert/getQuote       Request a conversion quote
POST   /sapi/v1/convert/acceptQuote    Accept a quote and execute conversion
GET    /sapi/v1/convert/orderStatus    Query conversion order status
GET    /sapi/v1/convert/tradeFlow      Trade history
```

---

## Rate Limit Note (from changelog 2024-10-14)

`POST /sapi/v1/convert/getQuote`:
- Rate limit: 360/hour, 500/day
- `validTime` parameter: can only be set to `10s`

---

## TradeAutopsy relevance

Convert is a read-only data source for behavioral context (e.g. user moved funds between assets via convert — affects position sizing signals). No convert actions are placed by TradeAutopsy. Read-only only: `GET /sapi/v1/convert/tradeFlow` if relevant to behavioral model.
