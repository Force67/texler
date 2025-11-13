#!/usr/bin/env bash

# Backend Test Runner for Texler
# This script runs comprehensive tests against the Rust backend

set -e  # Exit on any error

echo "🧪 Texler Backend Test Runner"
echo "============================="
echo

# Function to check if backend is running
check_backend() {
    echo "🔍 Checking if backend is running..."
    if curl -s http://localhost:8080/health > /dev/null; then
        echo "✅ Backend is running"
        return 0
    else
        echo "❌ Backend is not running"
        return 1
    fi
}

# Function to check if Docker services are running
check_docker() {
    echo "🐳 Checking Docker services..."
    if docker compose ps | grep -q "Up"; then
        echo "✅ Docker services are running"
        return 0
    else
        echo "❌ Docker services are not running"
        return 1
    fi
}

# Function to start Docker services
start_docker() {
    echo "🚀 Starting Docker services..."
    docker compose up -d
    echo "⏳ Waiting for services to be ready..."

    # Wait for backend to be ready
    for i in {1..30}; do
        if curl -s http://localhost:8080/health > /dev/null; then
            echo "✅ Backend is ready!"
            break
        fi
        echo "   Waiting for backend... ($i/30)"
        sleep 2
    done
}

# Main test execution
run_tests() {
    echo
    echo "🔧 Running Backend Tests"
    echo "======================="
    echo

    # Build the test
    echo "📦 Building tests..."
    if ! cargo test --no-run --test api_tests; then
        echo "❌ Failed to build tests"
        exit 1
    fi
    echo "✅ Tests built successfully"
    echo

    # Run the tests
    echo "🧪 Executing API tests..."
    cargo test --test api_tests -- --nocapture
    echo

    echo "🎉 Backend tests completed!"
}

# Check dependencies
check_dependencies() {
    echo "🔧 Checking dependencies..."

    if ! command -v cargo &> /dev/null; then
        echo "❌ Rust/Cargo is not installed"
        exit 1
    fi

    if ! command -v docker &> /dev/null; then
        echo "❌ Docker is not installed"
        exit 1
    fi

    if ! command -v curl &> /dev/null; then
        echo "❌ curl is not installed"
        exit 1
    fi

    echo "✅ All dependencies are available"
}

# Main script logic
main() {
    # Check dependencies first
    check_dependencies
    echo

    # Check if Docker services are running
    if ! check_docker; then
        echo
        read -p "Would you like to start Docker services? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            start_docker
        else
            echo "❌ Cannot proceed without running services"
            exit 1
        fi
    fi

    # Check if backend is running
    if ! check_backend; then
        echo
        read -p "Backend is not ready. Wait for it to start? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo "⏳ Waiting 10 seconds for backend to start..."
            sleep 10
            if ! check_backend; then
                echo "❌ Backend is still not running. Please check the logs."
                exit 1
            fi
        else
            echo "❌ Cannot proceed without backend running"
            exit 1
        fi
    fi

    # Run the tests
    run_tests
}

# Handle script arguments
case "${1:-}" in
    --check-only)
        check_dependencies
        check_docker
        check_backend
        echo "✅ All checks passed"
        ;;
    --start-services)
        check_dependencies
        start_docker
        ;;
    --help|-h)
        echo "Usage: $0 [OPTIONS]"
        echo
        echo "Options:"
        echo "  --check-only    Only check if services are running"
        echo "  --start-services Start Docker services and exit"
        echo "  --help          Show this help message"
        echo
        echo "No arguments: Run full test suite"
        ;;
    "")
        main
        ;;
    *)
        echo "Unknown option: $1"
        echo "Use --help for usage information"
        exit 1
        ;;
esac