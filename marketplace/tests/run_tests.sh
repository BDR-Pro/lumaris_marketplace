#!/bin/bash

# Run all tests with pytest-asyncio and coverage
# This script ensures all tests run properly in the conda environment

# Set environment variables for testing
export PYTHONPATH=.
export TESTING=1

# Install dependencies if needed
if [ "$1" == "--install" ]; then
    echo "Installing dependencies..."
    conda env update -f environment.yml
fi

# Run the tests
echo "Running tests with pytest-asyncio..."
python -m pytest \
    --asyncio-mode=auto \
    --cov=marketplace \
    --cov-report=term \
    --cov-report=html:coverage_html \
    -v \
    "$@"

# Check the exit code
if [ $? -eq 0 ]; then
    echo "All tests passed! 🎉"
else
    echo "Some tests failed. 😢"
    exit 1
fi

