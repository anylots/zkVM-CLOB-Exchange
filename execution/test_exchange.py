#!/usr/bin/env python3
"""
Test script for the minimal order-book exchange
"""

import requests
import json
import time

BASE_URL = "http://[::1]:3030"

def test_deposit():
    """Test depositing tokens"""
    print("=== Testing Deposits ===")
    
    # Deposit ETH for user1
    response = requests.post(f"{BASE_URL}/deposit", json={
        "user_id": "user1",
        "token": "ETH",
        "amount": 1000000  # 1 ETH (in micro units)
    })
    print(f"Deposit ETH for user1: {response.json()}")
    
    # Deposit USDT for user1
    response = requests.post(f"{BASE_URL}/deposit", json={
        "user_id": "user1",
        "token": "USDT",
        "amount": 5000000000  # 5000 USDT (in micro units)
    })
    print(f"Deposit USDT for user1: {response.json()}")
    
    # Deposit ETH for user2
    response = requests.post(f"{BASE_URL}/deposit", json={
        "user_id": "user2",
        "token": "ETH",
        "amount": 2000000  # 2 ETH
    })
    print(f"Deposit ETH for user2: {response.json()}")
    
    # Deposit USDT for user2
    response = requests.post(f"{BASE_URL}/deposit", json={
        "user_id": "user2",
        "token": "USDT",
        "amount": 10000000000  # 10000 USDT
    })
    print(f"Deposit USDT for user2: {response.json()}")

def test_balances():
    """Test checking balances"""
    print("\n=== Testing Balance Queries ===")
    
    # Check user1 ETH balance
    response = requests.post(f"{BASE_URL}/balance", json={
        "user_id": "user1",
        "token": "ETH"
    })
    print(f"User1 ETH balance: {response.json()}")
    
    # Check user1 USDT balance
    response = requests.post(f"{BASE_URL}/balance", json={
        "user_id": "user1",
        "token": "USDT"
    })
    print(f"User1 USDT balance: {response.json()}")
    
    # Check user2 ETH balance
    response = requests.post(f"{BASE_URL}/balance", json={
        "user_id": "user2",
        "token": "ETH"
    })
    print(f"User2 ETH balance: {response.json()}")

def test_orders():
    """Test placing and matching orders"""
    print("\n=== Testing Order Placement ===")
    
    # User1 places a buy order (wants to buy ETH with USDT)
    response = requests.post(f"{BASE_URL}/order/place", json={
        "user_id": "user1",
        "pair_id": "ETH_USDT",
        "amount": 500000,  # 0.5 ETH
        "price": 3000000000,  # 3000 USDT per ETH (in micro units)
        "side": True  # Buy order
    })
    buy_order = response.json()
    print(f"User1 buy order: {buy_order}")
    
    # User2 places a sell order (wants to sell ETH for USDT)
    response = requests.post(f"{BASE_URL}/order/place", json={
        "user_id": "user2",
        "pair_id": "ETH_USDT",
        "amount": 300000,  # 0.3 ETH
        "price": 2900000000,  # 2900 USDT per ETH (should match with buy order)
        "side": False  # Sell order
    })
    sell_order = response.json()
    print(f"User2 sell order: {sell_order}")
    
    # Place another sell order that won't match
    response = requests.post(f"{BASE_URL}/order/place", json={
        "user_id": "user2",
        "pair_id": "ETH_USDT",
        "amount": 200000,  # 0.2 ETH
        "price": 3100000000,  # 3100 USDT per ETH (higher than buy order)
        "side": False  # Sell order
    })
    no_match_order = response.json()
    print(f"User2 no-match sell order: {no_match_order}")
    
    return buy_order, sell_order, no_match_order

def test_order_book():
    """Test order book queries"""
    print("\n=== Testing Order Book ===")
    
    response = requests.post(f"{BASE_URL}/orderbook", json={
        "pair_id": "ETH_USDT"
    })
    print(f"ETH_USDT order book: {response.json()}")

def test_trades():
    """Test trade history"""
    print("\n=== Testing Trade History ===")
    
    response = requests.post(f"{BASE_URL}/trades", json={})
    print(f"All trades: {response.json()}")

def test_order_cancellation(order_data):
    """Test order cancellation"""
    print("\n=== Testing Order Cancellation ===")
    
    if order_data and order_data.get('success') and order_data.get('data'):
        order_id = order_data['data']['order']['id']
        response = requests.post(f"{BASE_URL}/order/cancel", json={
            "pair_id": "ETH_USDT",
            "order_id": order_id
        })
        print(f"Cancel order {order_id}: {response.json()}")

def test_withdrawal():
    """Test token withdrawal"""
    print("\n=== Testing Withdrawals ===")
    
    # Try to withdraw some ETH
    response = requests.post(f"{BASE_URL}/withdraw", json={
        "user_id": "user1",
        "token": "ETH",
        "amount": 100000  # 0.1 ETH
    })
    print(f"Withdraw ETH for user1: {response.json()}")
    
    # Try to withdraw more than balance (should fail)
    response = requests.post(f"{BASE_URL}/withdraw", json={
        "user_id": "user1",
        "token": "ETH",
        "amount": 10000000  # 10 ETH (more than balance)
    })
    print(f"Withdraw too much ETH (should fail): {response.json()}")

def main():
    """Run all tests"""
    print("Testing Minimal Order Book Exchange")
    print("=" * 50)
    
    try:
        # Test basic functionality
        test_deposit()
        test_balances()
        
        # Test order placement and matching
        buy_order, sell_order, no_match_order = test_orders()
        
        # Test order book and trades
        test_order_book()
        test_trades()
        
        # Test order cancellation
        test_order_cancellation(no_match_order)
        
        # Test withdrawals
        test_withdrawal()
        
        # Final balance check
        print("\n=== Final Balance Check ===")
        test_balances()
        
        print("\n" + "=" * 50)
        print("All tests completed!")
        
    except requests.exceptions.ConnectionError:
        print("Error: Could not connect to the server. Make sure the server is running on http://[::1]:3030")
    except Exception as e:
        print(f"Error during testing: {e}")

if __name__ == "__main__":
    main()
