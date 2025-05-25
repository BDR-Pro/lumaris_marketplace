# Setting Up Conda Environment for Testing

This guide explains how to set up and use the Conda environment for running tests in the Lumaris Marketplace project.

## Prerequisites

- [Miniconda](https://docs.conda.io/en/latest/miniconda.html) or [Anaconda](https://www.anaconda.com/products/distribution) installed on your system

## Creating the Conda Environment

1. Navigate to the project root directory:
   ```bash
   cd lumaris_marketplace
   ```

2. Create the Conda environment from the environment.yml file:
   ```bash
   conda env create -f environment.yml
   ```

3. Activate the environment:
   ```bash
   conda activate lumaris-marketplace
   ```

## Running Tests

### Using the Test Script

We've provided a convenient script to run all tests with the correct settings:

```bash
cd marketplace
./tests/run_tests.sh
```

This script will:
- Set the correct Python path
- Run pytest with asyncio support
- Generate coverage reports

### Options

You can pass additional pytest options to the script:

```bash
# Run a specific test file
./tests/run_tests.sh tests/test_websocket.py

# Run with verbose output
./tests/run_tests.sh -v

# Run tests matching a pattern
./tests/run_tests.sh -k "websocket"

# Update the conda environment before running tests
./tests/run_tests.sh --install
```

### Running Tests Manually

If you prefer to run pytest directly:

```bash
cd marketplace
python -m pytest --asyncio-mode=auto
```

## Troubleshooting

### Missing Dependencies

If you encounter missing dependencies, update your conda environment:

```bash
conda env update -f environment.yml
```

### WebSocket Test Failures

If WebSocket tests are failing:

1. Make sure pytest-asyncio is installed:
   ```bash
   conda install pytest-asyncio
   ```

2. Verify that the WebSocket server is properly configured in the test environment:
   ```bash
   python -c "import websockets; print(websockets.__version__)"
   ```

3. Check that the JWT authentication is working:
   ```bash
   python -c "import jose; print(jose.__version__)"
   ```

### Database Test Failures

If database tests are failing:

1. Ensure SQLAlchemy is properly installed:
   ```bash
   python -c "import sqlalchemy; print(sqlalchemy.__version__)"
   ```

2. Verify that the test database configuration is correct in conftest.py

## Updating the Environment

To update the conda environment with new dependencies:

```bash
conda env update -f environment.yml
```

