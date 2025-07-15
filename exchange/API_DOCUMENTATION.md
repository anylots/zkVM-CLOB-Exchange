# Minimal Order Book Exchange API Documentation

This is a minimal order-book exchange implementation in Rust that supports deposits, withdrawals, limit orders, order matching, and order cancellation.

## Server Information

- **Base URL**: `http://[::1]:3030`
- **Protocol**: HTTP POST requests with JSON payloads
- **Response Format**: All responses follow the format:
  ```json
  {
    "success": boolean,
    "data": object | null,
    "error": string | null
  }
  ```

## API Endpoints

### 1. Deposit Tokens

**Endpoint**: `POST /deposit`

**Description**: Deposit ERC-20 style tokens to a user's account.

**Request Body**:
```json
{
  "user_id": "string",
  "token": "string",
  "amount": number
}
```

**Example**:
```bash
curl -X POST http://[::1]:3030/deposit \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user1",
    "token": "ETH",
    "amount": 1000000
  }'
```

### 2. Withdraw Tokens

**Endpoint**: `POST /withdraw`

**Description**: Withdraw tokens from a user's account.

**Request Body**:
```json
{
  "user_id": "string",
  "token": "string",
  "amount": number
}
```

**Example**:
```bash
curl -X POST http://[::1]:3030/withdraw \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user1",
    "token": "ETH",
    "amount": 100000
  }'
```

### 3. Check Balance

**Endpoint**: `POST /balance`

**Description**: Check a user's token balance.

**Request Body**:
```json
{
  "user_id": "string",
  "token": "string"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "balance": number
  },
  "error": null
}
```

### 4. Place Order

**Endpoint**: `POST /order/place`

**Description**: Place a limit order (buy or sell).

**Request Body**:
```json
{
  "user_id": "string",
  "pair_id": "string",
  "amount": number,
  "price": number,
  "side": boolean
}
```

**Parameters**:
- `pair_id`: Trading pair in format "BASE/QUOTE" (e.g., "ETH_USDT")
- `amount`: Amount of base token to buy/sell
- `price`: Price per unit of base token in quote token
- `side`: `true` for buy order, `false` for sell order

**Response**:
```json
{
  "success": true,
  "data": {
    "order": {
      "id": "string",
      "user_id": "string",
      "pair_id": "string",
      "amount": number,
      "filled_amount": number,
      "price": number,
      "side": boolean,
      "status": "string",
      "created_at": number,
      "updated_at": number
    },
    "trades": [
      {
        "buy_order_id": "string",
        "sell_order_id": "string",
        "price": number,
        "quantity": number,
        "timestamp": number
      }
    ]
  },
  "error": null
}
```

### 5. Cancel Order

**Endpoint**: `POST /order/cancel`

**Description**: Cancel an existing order.

**Request Body**:
```json
{
  "pair_id": "string",
  "order_id": "string"
}
```

### 6. Get Order

**Endpoint**: `POST /order/get`

**Description**: Get details of a specific order.

**Request Body**:
```json
{
  "pair_id": "string",
  "order_id": "string"
}
```

### 7. Get Order Book

**Endpoint**: `POST /orderbook`

**Description**: Get the current order book state for a trading pair.

**Request Body**:
```json
{
  "pair_id": "string"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "best_bid": number | null,
    "best_ask": number | null
  },
  "error": null
}
```

### 8. Get Trade History

**Endpoint**: `POST /trades`

**Description**: Get all executed trades.

**Request Body**: `{}`

**Response**:
```json
{
  "success": true,
  "data": [
    {
      "buy_order_id": "string",
      "sell_order_id": "string",
      "price": number,
      "quantity": number,
      "timestamp": number
    }
  ],
  "error": null
}
```

## Features

### ✅ Deposits & Withdrawals
- Support for two ERC-20 style tokens
- Balance validation for withdrawals
- Simple account management

### ✅ Limit Orders
- Buy and sell orders with price and quantity
- Automatic order ID generation
- Order status tracking (Pending, PartiallyFilled, Filled, Cancelled)

### ✅ Order Matching
- Price-time priority matching algorithm
- Partial fills supported
- Immediate execution when orders cross
- Best bid/ask tracking

### ✅ Order Cancellation
- Cancel pending orders
- Status updates to "Cancelled"
- Automatic removal from order book

## Data Types

### Order Status
- `Pending`: Order is in the order book waiting for a match
- `PartiallyFilled`: Order has been partially executed
- `Filled`: Order has been completely executed
- `Cancelled`: Order has been cancelled

### Order Side
- `true`: Buy order (bid)
- `false`: Sell order (ask)

## Example Trading Scenario

1. **Setup**: Users deposit tokens
   ```bash
   # User1 deposits ETH and USDT
   curl -X POST http://[::1]:3030/deposit -H "Content-Type: application/json" \
     -d '{"user_id": "user1", "token": "ETH", "amount": 1000000}'
   curl -X POST http://[::1]:3030/deposit -H "Content-Type: application/json" \
     -d '{"user_id": "user1", "token": "USDT", "amount": 5000000000}'
   ```

2. **Place Orders**: Users place buy/sell orders
   ```bash
   # User1 wants to buy 0.5 ETH at 3000 USDT per ETH
   curl -X POST http://[::1]:3030/order/place -H "Content-Type: application/json" \
     -d '{"user_id": "user1", "pair_id": "ETH_USDT", "amount": 500000, "price": 3000000000, "side": true}'
   
   # User2 wants to sell 0.3 ETH at 2900 USDT per ETH (will match!)
   curl -X POST http://[::1]:3030/order/place -H "Content-Type: application/json" \
     -d '{"user_id": "user2", "pair_id": "ETH_USDT", "amount": 300000, "price": 2900000000, "side": false}'
   ```

3. **Check Results**: View trades and order book
   ```bash
   # Check executed trades
   curl -X POST http://[::1]:3030/trades -H "Content-Type: application/json" -d '{}'
   
   # Check current order book
   curl -X POST http://[::1]:3030/orderbook -H "Content-Type: application/json" \
     -d '{"pair_id": "ETH_USDT"}'
   ```

## Running the Server

```bash
# Start the server
cargo run

# The server will start on http://[::1]:3030
# Use the test script to verify functionality
python3 test_exchange.py
```

## Notes

- All amounts are in micro units (1 ETH = 1,000,000 micro units)
- Prices are also in micro units for precision
- The matching engine uses price-time priority
- Orders are matched immediately when placed if there's a cross
- The implementation is minimal and suitable for educational purposes
