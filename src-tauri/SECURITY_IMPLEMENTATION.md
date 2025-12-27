# TTRPG Assistant Security Implementation

## Overview

This document outlines the comprehensive security features implemented in Task 23.8 for the TTRPG Assistant desktop application. The implementation follows OWASP security guidelines and implements a defense-in-depth approach to protect sensitive user data, API keys, and campaign information.

## Security Features Implemented

### 1. Process Sandboxing (`src/security/sandbox.rs`)

**Features:**
- Resource limits (CPU, memory, file descriptors)
- Filesystem access controls with path validation
- Network restrictions with domain/IP filtering
- Command validation and argument sanitization
- Process privilege reduction (Unix systems)
- Real-time resource monitoring
- Automatic violation detection and logging

**Key Security Measures:**
- Blocks dangerous commands (rm, del, chmod, etc.)
- Prevents path traversal attacks
- Enforces memory and CPU usage limits
- Restricts access to system directories (/dev, /proc, /sys)
- Validates working directories against allowlists

### 2. Enhanced Content Security Policy

**Updated CSP in `tauri.conf.json`:**
```json
"csp": "default-src 'self' tauri: asset:; script-src 'self' tauri: 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline' tauri: asset:; connect-src 'self' ipc: tauri: asset: http://127.0.0.1:* https://api.openai.com https://api.anthropic.com; img-src 'self' data: blob: tauri: asset: https:; font-src 'self' data: tauri: asset:; media-src 'self' tauri: asset:; worker-src 'self' tauri: asset:; child-src 'none'; frame-src 'none'; object-src 'none'; base-uri 'self'; form-action 'none'; upgrade-insecure-requests; block-all-mixed-content; require-trusted-types-for 'script'; trusted-types default dompurify;"
```

**Security Enhancements:**
- Prevents XSS attacks with strict source controls
- Blocks mixed content and insecure requests
- Implements Trusted Types for script execution
- Restricts frame embedding and object sources
- Allows only necessary external connections (AI APIs)

### 3. OS Keychain Integration (`src/security/keychain.rs`)

**Features:**
- Native keychain support for Windows, macOS, and Linux
- Encrypted fallback storage for unsupported platforms
- Automatic credential expiration
- Metadata management for stored credentials
- Secure credential lifecycle management

**Supported Platforms:**
- Windows: Credential Manager API
- macOS: Keychain Services
- Linux: Secret Service API (libsecret)
- Fallback: AES-256-GCM encrypted file storage

### 4. Comprehensive Audit Logging (`src/security/audit.rs`)

**Features:**
- Tamper-resistant log storage with cryptographic integrity
- Structured JSON logging with full event details
- Automatic log rotation and compression
- Configurable retention policies
- Hash chain verification for integrity checking
- Encrypted log storage option

**Log Categories:**
- Authentication events
- Authorization decisions
- Input validation failures
- Process sandboxing violations
- Resource usage anomalies
- Security policy violations
- Threat detection alerts

### 5. Granular Permission Management (`src/security/permissions.rs`)

**Features:**
- Role-based access control (RBAC)
- Resource-specific permissions
- Permission inheritance through role hierarchy
- Dynamic permission evaluation
- Cycle detection in role hierarchies
- User permission caching for performance

**Default Roles:**
- Administrator: Full system access
- User: Standard campaign and character operations
- Read-only: View-only access to data

### 6. Input Validation and Sanitization (`src/security/validation.rs`)

**Features:**
- Comprehensive input validation engine
- Schema-based validation rules
- Command injection prevention
- Path traversal protection
- Email and URL format validation
- JSON structure validation
- Custom sanitization functions

**Validation Types:**
- String validation with length and pattern checks
- Email format validation
- URL safety validation
- File path security validation
- Command argument validation
- JSON schema validation

### 7. Security Monitoring and Alerting (`src/security/monitoring.rs`)

**Features:**
- Real-time threat detection
- Behavioral anomaly detection
- Resource usage monitoring
- Pattern-based threat identification
- Automatic alert generation
- Security metrics collection
- Configurable threat thresholds

**Threat Detection:**
- Brute force authentication attempts
- Command injection attempts
- Resource exhaustion attacks
- Anomalous access patterns
- Privilege escalation attempts

### 8. Cryptographic Operations (`src/security/crypto.rs`)

**Features:**
- Secure random number generation
- Multiple hash algorithms (SHA-256, SHA-512, BLAKE3)
- Digital signatures (HMAC-SHA256, Ed25519, ECDSA)
- Key derivation functions (PBKDF2, Argon2)
- Time-based OTP generation (TOTP)
- Session token management
- Constant-time comparisons

## Integration Points

### 1. Main Application Integration

The security system is integrated into the main Tauri application through:

```rust
// State management
.manage(SecurityManagerState::new())

// Command handlers
security_commands::initialize_security_manager,
security_commands::create_security_session,
security_commands::validate_session_permission,
// ... additional security commands
```

### 2. Secure MCP Bridge

Enhanced MCP communication with security validation:

```rust
// Secure MCP calls with validation
secure_mcp_bridge::secure_mcp_call,
secure_mcp_bridge::validate_mcp_method,
secure_mcp_bridge::get_secure_mcp_stats,
```

### 3. Data Manager Integration

Security is integrated with the existing data manager for:
- Encrypted data storage
- Secure backup operations
- Integrity verification
- Access control for data operations

## Usage Examples

### 1. Initialize Security Manager

```javascript
// Initialize security system
await invoke('initialize_security_manager');

// Create user session with permissions
const sessionId = await invoke('create_security_session', {
    permissions: ['campaign.read', 'campaign.create', 'character.read']
});
```

### 2. Validate Inputs

```javascript
// Validate user input
const result = await invoke('validate_input', {
    fieldName: 'campaign_name',
    value: userInput,
    context: {}
});

// Sanitize string input
const sanitized = await invoke('sanitize_string', {
    input: untrustedString
});
```

### 3. Store Credentials Securely

```javascript
// Store API key in OS keychain
await invoke('store_credential', {
    service: 'openai_api',
    account: 'user@example.com',
    secret: apiKey,
    additionalData: { endpoint: 'https://api.openai.com' },
    description: 'OpenAI API Key for TTRPG Assistant'
});

// Retrieve credential
const credential = await invoke('retrieve_credential', {
    service: 'openai_api',
    account: 'user@example.com'
});
```

### 4. Create Sandboxed Process

```javascript
// Create sandboxed Python process
const processId = await invoke('create_sandboxed_process', {
    command: 'python',
    args: ['mcp_server.py'],
    workingDir: '/app/data',
    sessionId: userSessionId
});

// Monitor process status
const status = await invoke('get_process_status', {
    processId: processId,
    sessionId: userSessionId
});
```

### 5. Monitor Security Events

```javascript
// Get security statistics
const stats = await invoke('get_security_stats');

// Get recent security alerts
const alerts = await invoke('get_security_alerts', {
    limit: 50
});

// Log custom security event
await invoke('log_security_event', {
    eventType: 'Authentication',
    severity: 'Medium',
    message: 'User login successful',
    details: { userId: 'user123', timestamp: Date.now() },
    sessionId: userSessionId
});
```

## Security Configuration

### Default Security Settings

```rust
SecurityConfig {
    audit_logging_enabled: true,
    audit_log_retention_days: 90,
    process_sandboxing_enabled: true,
    input_validation_enabled: true,
    security_monitoring_enabled: true,
    resource_monitoring_enabled: true,
    max_subprocess_memory_mb: 512,
    max_subprocess_cpu_percent: 50.0,
    keychain_integration_enabled: true,
    // ... additional configuration
}
```

### Customizable Thresholds

- Memory usage thresholds for alerts
- CPU usage limits per subprocess
- File operation rate limits
- Network request rate limits
- Failed authentication attempt limits

## Security Compliance

### OWASP Guidelines Followed

1. **A01: Broken Access Control** - Implemented RBAC and session-based permissions
2. **A02: Cryptographic Failures** - Strong encryption with proper key management
3. **A03: Injection** - Comprehensive input validation and sanitization
4. **A04: Insecure Design** - Security-by-design with defense-in-depth
5. **A05: Security Misconfiguration** - Secure defaults and configuration validation
6. **A06: Vulnerable Components** - Regular dependency updates and security scanning
7. **A07: Authentication Failures** - Session management and brute force protection
8. **A08: Software Integrity Failures** - Code signing and integrity verification
9. **A09: Logging Failures** - Comprehensive audit logging with tamper protection
10. **A10: Server-Side Request Forgery** - URL validation and allowlist controls

### Additional Security Measures

- Regular security event cleanup and archival
- Automatic session expiration
- Resource usage monitoring and alerting
- Process isolation and sandboxing
- Cryptographic integrity verification
- Real-time threat detection and response

## Testing and Validation

The security implementation includes comprehensive test coverage for:

- Input validation edge cases
- Permission system integrity
- Cryptographic operation correctness
- Sandbox containment effectiveness
- Audit log integrity verification
- Threat detection accuracy

## Future Enhancements

Recommended future security improvements:

1. **Advanced Threat Detection**: Machine learning-based anomaly detection
2. **Zero Trust Architecture**: Enhanced identity verification and continuous validation
3. **Security Orchestration**: Automated incident response and remediation
4. **Advanced Sandboxing**: Container-based process isolation
5. **Hardware Security**: TPM/HSM integration for key storage
6. **Network Security**: Advanced firewall and intrusion detection

## Conclusion

The implemented security system provides enterprise-grade protection for the TTRPG Assistant application while maintaining usability and performance. The defense-in-depth approach ensures multiple layers of protection against common attack vectors, and the comprehensive logging and monitoring capabilities provide visibility into security events and potential threats.

The modular design allows for easy extension and customization of security features as requirements evolve, and the integration with existing application components ensures seamless operation without disrupting user workflows.