#!/bin/bash

echo "Checking NEAR testnet balance for cuteharbor3573.testnet..."
echo "=================================================="

while true; do
    BALANCE=$(near state cuteharbor3573.testnet --networkId testnet 2>/dev/null | grep formattedAmount | awk '{print $2}' | tr -d "'")
    TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
    
    if [ -z "$BALANCE" ]; then
        echo "[$TIMESTAMP] Error checking balance"
    else
        echo "[$TIMESTAMP] Balance: $BALANCE NEAR"
        
        # Check if balance is greater than 5 NEAR
        if (( $(echo "$BALANCE > 5" | bc -l) )); then
            echo "✅ Sufficient balance detected! You now have more than 5 NEAR."
            echo "Ready to continue with deployment!"
            break
        fi
    fi
    
    echo "Waiting 10 seconds before next check..."
    echo "Press Ctrl+C to stop monitoring"
    echo "------------------------------------------"
    sleep 10
done