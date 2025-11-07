SERVER_URL="http://trustify-keycloak:8080"
USERNAME="admin"
PASSWORD="admin"
REALM="master"

# Login
/opt/keycloak/bin/kcadm.sh config credentials \
--server ${SERVER_URL} \
--user ${USERNAME} \
--password ${PASSWORD} \
--realm ${REALM}

# Start working on Trustify Realm
TRUSTIFY_REALM="trustd"
TRUSTIFY_ROLE="admin"
TRUSTIFY_USERNAME="admin"

# Realm
/opt/keycloak/bin/kcadm.sh create realms -s realm=${TRUSTIFY_REALM} -s enabled=true

# Realm roles
/opt/keycloak/bin/kcadm.sh create roles -r ${TRUSTIFY_REALM} -s name=${TRUSTIFY_ROLE} -o
admin_role_id=$(/opt/keycloak/bin/kcadm.sh get roles -r "${TRUSTIFY_REALM}" --fields id,name --format csv --noquotes | grep ",${TRUSTIFY_ROLE}" | sed 's/,.*//')

# Scopes
for scope in read:document create:document update:document delete:document; do
    /opt/keycloak/bin/kcadm.sh create client-scopes -r "${TRUSTIFY_REALM}" -s "name=$scope" -s protocol=openid-connect    
done

# Roles scope mappings
for scope in read:document create:document update:document delete:document; do
    scope_id=$(/opt/keycloak/bin/kcadm.sh get client-scopes -r "${TRUSTIFY_REALM}" --fields id,name --format csv --noquotes | grep ",${scope}" | sed 's/,.*//')
    /opt/keycloak/bin/kcadm.sh create "client-scopes/${scope_id}/scope-mappings/realm" -r "${TRUSTIFY_REALM}" -b '[{"name":"'"${TRUSTIFY_ROLE}"'", "id":"'"${admin_role_id}"'"}]'
done

# Users
/opt/keycloak/bin/kcadm.sh create users -r ${TRUSTIFY_REALM} -s username=${TRUSTIFY_USERNAME} -s enabled=true -s firstName="admin" -s lastName="admin" -s email="admin@trustify.org" -o
/opt/keycloak/bin/kcadm.sh set-password -r ${TRUSTIFY_REALM} --username admin --new-password admin

/opt/keycloak/bin/kcadm.sh add-roles -r ${TRUSTIFY_REALM} --uusername ${TRUSTIFY_USERNAME} --rolename ${TRUSTIFY_ROLE}

# Clients
/opt/keycloak/bin/kcadm.sh create clients -r ${TRUSTIFY_REALM} -f - << EOF
{
  "clientId": "ui",
  "publicClient": true,
  "webOrigins": [
    "*"
  ],
  "redirectUris": [
    "*"
  ],
  "defaultClientScopes": [
    "acr",
    "basic",
    "email",
    "profile",
    "roles",
    "create:document",
    "read:document",
    "update:document",
    "delete:document"
  ],
  "optionalClientScopes": [
    "address",
    "microprofile-jwt",
    "offline_access",
    "phone"
  ]
}
EOF

/opt/keycloak/bin/kcadm.sh create clients -r ${TRUSTIFY_REALM} -f - << EOF
{
  "clientId": "cli",
  "publicClient": false,
  "standardFlowEnabled": false,
  "serviceAccountsEnabled": true,
  "secret": "secret",
  "defaultClientScopes": [
    "acr",
    "basic",
    "email",
    "profile",
    "roles",
    "create:document",
    "read:document",
    "update:document",
    "delete:document"
  ],
  "optionalClientScopes": [
    "address",
    "microprofile-jwt",
    "offline_access",
    "phone"
  ]
}
EOF

# Assign roles to service-account
cli_client="cli"

adminRoleId=$(/opt/keycloak/bin/kcadm.sh get roles -r ${TRUSTIFY_REALM} --fields id,name --format csv --noquotes | grep ",${TRUSTIFY_ROLE}" | sed 's/,.*//')

cliClientId=$(/opt/keycloak/bin/kcadm.sh get clients -r ${TRUSTIFY_REALM} --fields id,clientId --format csv --noquotes | grep ",${cli_client}" | sed 's/,.*//')
serviceAccountId=$(/opt/keycloak/bin/kcadm.sh get clients/${cliClientId}/service-account-user -r ${TRUSTIFY_REALM} --fields id,username --format csv --noquotes | grep ",service-account-${cli_client}" | sed 's/,.*//')

/opt/keycloak/bin/kcadm.sh create users/${serviceAccountId}/role-mappings/realm -r ${TRUSTIFY_REALM} -f - << EOF
[
  {
    "id": "${adminRoleId}",
    "name": "${TRUSTIFY_ROLE}",
    "clientRole": false,
    "composite": false
  }
]
EOF