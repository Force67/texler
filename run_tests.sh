#!/bin/bash

# Comprehensive Test Runner for Texler Backend
# This script starts the Docker services and runs all tests

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
COMPOSE_FILE="docker-compose.yml"
API_BASE_URL="http://localhost:8080"
DB_HOST="localhost"
DB_PORT="5432"
DB_NAME="texler"
DB_USER="postgres"
DB_PASSWORD="password"
WAIT_TIMEOUT=60
TEST_RETENTION_DAYS=7

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "\n${BLUE}=== $1 ===${NC}"
}

# Check if Docker and Docker Compose are available
check_dependencies() {
    log_step "Checking Dependencies"

    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed or not in PATH"
        exit 1
    fi

    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        log_error "Docker Compose is not installed or not in PATH"
        exit 1
    fi

    if ! command -v python3 &> /dev/null; then
        log_error "Python 3 is not installed or not in PATH"
        exit 1
    fi

    if ! command -v pip3 &> /dev/null; then
        log_error "pip3 is not installed or not in PATH"
        exit 1
    fi

    log_success "All dependencies are available"
}

# Install Python test dependencies
install_test_dependencies() {
    log_step "Installing Test Dependencies"

    if [ ! -f "test_requirements.txt" ]; then
        log_error "test_requirements.txt not found"
        exit 1
    fi

    pip3 install -r test_requirements.txt
    log_success "Test dependencies installed"
}

# Check if Docker containers are already running
check_containers_running() {
    log_step "Checking Running Containers"

    if docker-compose -f $COMPOSE_FILE ps | grep -q "Up"; then
        log_warning "Some containers are already running"
        log_info "Stopping existing containers..."
        docker-compose -f $COMPOSE_FILE down
        sleep 5
    fi

    log_success "No conflicting containers running"
}

# Start Docker services
start_services() {
    log_step "Starting Docker Services"

    log_info "Starting PostgreSQL, Redis, LaTeX service, and Backend..."
    docker-compose -f $COMPOSE_FILE up -d postgres redis latex backend

    log_info "Waiting for services to be ready..."
    sleep 10
}

# Wait for backend to be healthy
wait_for_backend() {
    log_step "Waiting for Backend API"

    local attempts=0
    local max_attempts=$WAIT_TIMEOUT

    while [ $attempts -lt $max_attempts ]; do
        if curl -f -s "$API_BASE_URL/health" > /dev/null 2>&1; then
            log_success "Backend API is ready!"
            return 0
        fi

        attempts=$((attempts + 1))
        echo -n "."
        sleep 1
    done

    echo
    log_error "Backend API failed to become ready within $WAIT_TIMEOUT seconds"
    return 1
}

# Wait for database to be ready
wait_for_database() {
    log_step "Waiting for Database"

    local attempts=0
    local max_attempts=$WAIT_TIMEOUT

    while [ $attempts -lt $max_attempts ]; do
        if docker exec texler-postgres pg_isready -U postgres > /dev/null 2>&1; then
            log_success "Database is ready!"
            return 0
        fi

        attempts=$((attempts + 1))
        echo -n "."
        sleep 1
    done

    echo
    log_error "Database failed to become ready within $WAIT_TIMEOUT seconds"
    return 1
}

# Check service health
check_service_health() {
    log_step "Checking Service Health"

    # Check backend health
    if curl -f -s "$API_BASE_URL/health" > /dev/null; then
        log_success "Backend API: Healthy"
    else
        log_error "Backend API: Unhealthy"
        return 1
    fi

    # Check LaTeX service health
    if curl -f -s "$API_BASE_URL/api/v1/latex/health" > /dev/null; then
        log_success "LaTeX Service: Healthy"
    else
        log_warning "LaTeX Service: Unhealthy or not responding"
    fi

    # Check database connection
    if docker exec texler-postgres pg_isready -U postgres > /dev/null; then
        log_success "PostgreSQL: Healthy"
    else
        log_error "PostgreSQL: Unhealthy"
        return 1
    fi

    # Check Redis connection
    if docker exec texler-redis redis-cli ping > /dev/null 2>&1; then
        log_success "Redis: Healthy"
    else
        log_warning "Redis: Unhealthy"
    fi
}

# Run database integration tests
run_database_tests() {
    log_step "Running Database Integration Tests"

    if [ ! -f "test_database_integration.py" ]; then
        log_error "test_database_integration.py not found"
        return 1
    fi

    python3 test_database_integration.py \
        --host "$DB_HOST" \
        --port "$DB_PORT" \
        --database "$DB_NAME" \
        --user "$DB_USER" \
        --password "$DB_PASSWORD"

    if [ $? -eq 0 ]; then
        log_success "Database tests passed"
        return 0
    else
        log_error "Database tests failed"
        return 1
    fi
}

# Run API endpoint tests
run_api_tests() {
    log_step "Running API Endpoint Tests"

    if [ ! -f "test_api_endpoints.py" ]; then
        log_error "test_api_endpoints.py not found"
        return 1
    fi

    python3 test_api_endpoints.py --url "$API_BASE_URL"

    if [ $? -eq 0 ]; then
        log_success "API tests passed"
        return 0
    else
        log_error "API tests failed"
        return 1
    fi
}

# Run additional LaTeX compilation tests
run_latex_tests() {
    log_step "Running LaTeX Compilation Tests"

    if [ -f "apps/web/test_compile.py" ]; then
        log_info "Found existing LaTeX compilation test"
        python3 apps/web/test_compile.py

        if [ $? -eq 0 ]; then
            log_success "LaTeX compilation tests passed"
        else
            log_warning "LaTeX compilation tests failed"
        fi
    else
        log_warning "No LaTeX compilation test found at apps/web/test_compile.py"
    fi
}

# Generate test report
generate_report() {
    log_step "Generating Test Report"

    local timestamp=$(date +"%Y%m%d_%H%M%S")
    local report_file="test_report_$timestamp.log"

    {
        echo "=== TEXLER BACKEND TEST REPORT ==="
        echo "Date: $(date)"
        echo "API Base URL: $API_BASE_URL"
        echo "Database: $DB_HOST:$DB_PORT/$DB_NAME"
        echo ""
        echo "Docker Container Status:"
        docker-compose -f $COMPOSE_FILE ps
        echo ""
        echo "Backend Logs (last 50 lines):"
        docker logs --tail 50 texler-backend 2>&1
        echo ""
        echo "Database Logs (last 20 lines):"
        docker logs --tail 20 texler-postgres 2>&1
    } > "$report_file"

    log_success "Test report generated: $report_file"
}

# Cleanup function
cleanup() {
    log_step "Cleaning Up"

    # Ask user if they want to stop containers
    echo
    read -p "Do you want to stop the Docker containers? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        log_info "Stopping Docker containers..."
        docker-compose -f $COMPOSE_FILE down
        log_success "Containers stopped"
    else
        log_info "Containers will continue running"
        log_info "Stop them later with: docker-compose -f $COMPOSE_FILE down"
    fi

    # Clean up old test reports
    log_info "Cleaning up old test reports (older than $TEST_RETENTION_DAYS days)..."
    find . -name "test_report_*.log" -type f -mtime +$TEST_RETENTION_DAYS -delete 2>/dev/null || true
}

# Show usage
show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --skip-db-tests    Skip database integration tests"
    echo "  --skip-api-tests   Skip API endpoint tests"
    echo "  --skip-latex-tests Skip LaTeX compilation tests"
    echo "  --db-only          Run only database tests"
    echo "  --api-only         Run only API tests"
    echo "  --cleanup-only     Only perform cleanup (stop containers)"
    echo "  --help             Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  API_BASE_URL      Base URL for API testing (default: http://localhost:8080)"
    echo "  DB_HOST          Database host (default: localhost)"
    echo "  DB_PORT          Database port (default: 5432)"
    echo "  DB_NAME          Database name (default: texler)"
    echo "  DB_USER          Database user (default: postgres)"
    echo "  DB_PASSWORD      Database password (default: password)"
    echo "  WAIT_TIMEOUT     Service wait timeout in seconds (default: 60)"
}

# Parse command line arguments
SKIP_DB_TESTS=false
SKIP_API_TESTS=false
SKIP_LATEX_TESTS=false
DB_ONLY=false
API_ONLY=false
CLEANUP_ONLY=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-db-tests)
            SKIP_DB_TESTS=true
            shift
            ;;
        --skip-api-tests)
            SKIP_API_TESTS=true
            shift
            ;;
        --skip-latex-tests)
            SKIP_LATEX_TESTS=true
            shift
            ;;
        --db-only)
            DB_ONLY=true
            shift
            ;;
        --api-only)
            API_ONLY=true
            shift
            ;;
        --cleanup-only)
            CLEANUP_ONLY=true
            shift
            ;;
        --help)
            show_usage
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Handle cleanup only mode
if [ "$CLEANUP_ONLY" = true ]; then
    cleanup
    exit 0
fi

# Handle mutually exclusive options
if [ "$DB_ONLY" = true ] && [ "$API_ONLY" = true ]; then
    log_error "--db-only and --api-only are mutually exclusive"
    exit 1
fi

if [ "$DB_ONLY" = true ]; then
    SKIP_API_TESTS=true
    SKIP_LATEX_TESTS=true
fi

if [ "$API_ONLY" = true ]; then
    SKIP_DB_TESTS=true
fi

# Main execution
main() {
    log_info "Starting Texler Backend Test Suite"

    # Set trap for cleanup on exit
    trap cleanup EXIT

    check_dependencies
    install_test_dependencies
    check_containers_running

    if [ "$SKIP_API_TESTS" = false ]; then
        start_services
        wait_for_backend
        wait_for_database
        check_service_health
    else
        # For database-only tests, just start database
        log_info "Starting only PostgreSQL for database tests..."
        docker-compose -f $COMPOSE_FILE up -d postgres
        wait_for_database
    fi

    # Run tests
    DB_TESTS_PASSED=true
    API_TESTS_PASSED=true
    LATEX_TESTS_PASSED=true

    if [ "$SKIP_DB_TESTS" = false ]; then
        run_database_tests || DB_TESTS_PASSED=false
    fi

    if [ "$SKIP_API_TESTS" = false ]; then
        run_api_tests || API_TESTS_PASSED=false
    fi

    if [ "$SKIP_LATEX_TESTS" = false ]; then
        run_latex_tests || LATEX_TESTS_PASSED=false
    fi

    generate_report

    # Final summary
    log_step "Test Suite Summary"

    if [ "$DB_TESTS_PASSED" = true ] && [ "$API_TESTS_PASSED" = true ]; then
        log_success "All critical tests passed! 🎉"
        exit 0
    else
        log_error "Some tests failed!"

        if [ "$DB_TESTS_PASSED" = false ]; then
            log_error "❌ Database tests failed"
        fi

        if [ "$API_TESTS_PASSED" = false ]; then
            log_error "❌ API tests failed"
        fi

        exit 1
    fi
}

# Run main function
main