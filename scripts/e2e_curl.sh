#!/usr/bin/env bash
set -euo pipefail

CONFIG_PATH="${1:-}"
if [[ -z "${CONFIG_PATH}" ]]; then
  echo "usage: $0 /path/to/config.json"
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required"
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required"
  exit 1
fi

BASE_URL="$(jq -r '.base_url' "$CONFIG_PATH")"
ADMIN_BASE_URL="$(jq -r '.admin_base_url // empty' "$CONFIG_PATH")"
USER_KEY="$(jq -r '.user_key' "$CONFIG_PATH")"
ADMIN_KEY="$(jq -r '.admin_key // empty' "$CONFIG_PATH")"
DELAY_SECS="$(jq -r '.delay_secs // 10' "$CONFIG_PATH")"
RUN_PROXY="$(jq -r 'if .run_proxy == null then true else .run_proxy end' "$CONFIG_PATH")"
RUN_ADMIN="$(jq -r 'if .run_admin == null then false else .run_admin end' "$CONFIG_PATH")"

if [[ -z "${BASE_URL}" || -z "${USER_KEY}" ]]; then
  echo "base_url and user_key are required in config"
  exit 1
fi

curl_fail() {
  local msg="$1"
  echo "ERROR: ${msg}" >&2
  exit 1
}

admin_curl() {
  local method="$1"
  local path="$2"
  local body="${3:-}"

  if [[ -z "${ADMIN_BASE_URL}" || -z "${ADMIN_KEY}" ]]; then
    return 0
  fi

  local url="${ADMIN_BASE_URL%/}${path}"
  local args=(--noproxy '*' -sS -f -X "${method}" -H "x-admin-key: ${ADMIN_KEY}")
  if [[ -n "${body}" ]]; then
    args+=(-H "content-type: application/json" --data "${body}")
  fi
  curl "${args[@]}" "${url}" >/dev/null
}

admin_curl_json() {
  local method="$1"
  local path="$2"
  local body="${3:-}"
  local url="${ADMIN_BASE_URL%/}${path}"
  local args=(--noproxy '*' -sS -f -X "${method}" -H "x-admin-key: ${ADMIN_KEY}")
  if [[ -n "${body}" ]]; then
    args+=(-H "content-type: application/json" --data "${body}")
  fi
  curl "${args[@]}" "${url}"
}

proxy_curl() {
  local method="$1"
  local path="$2"
  local key_source="$3"
  local body="${4:-}"
  local extra_headers="${5:-}"
  local allow_redirect="${6:-false}"

  local url="${BASE_URL%/}${path}"
  local args=(--noproxy '*' -sS -f -X "${method}")
  local auth_header=()

  case "${key_source}" in
    x_api_key)
      auth_header+=(-H "x-api-key: ${USER_KEY}")
      ;;
    x_goog_api_key)
      auth_header+=(-H "x-goog-api-key: ${USER_KEY}")
      ;;
    query_key)
      if [[ "${url}" == *"?"* ]]; then
        url="${url}&key=${USER_KEY}"
      else
        url="${url}?key=${USER_KEY}"
      fi
      ;;
    *)
      auth_header+=(-H "authorization: Bearer ${USER_KEY}")
      ;;
  esac

  if [[ -n "${extra_headers}" ]]; then
    auth_header+=(-H "${extra_headers}")
  fi

  if [[ "${allow_redirect}" == "true" ]]; then
    args+=(-L)
  fi

  if [[ -n "${body}" ]]; then
    args+=(-H "content-type: application/json" --data "${body}")
  fi

  curl "${args[@]}" "${auth_header[@]}" "${url}" >/dev/null
}

jq_bool_default() {
  local filter="$1"
  local default_value="$2"
  jq -r "(${filter}) as \$v | if \$v == null then ${default_value} else \$v end" "$CONFIG_PATH"
}

run_admin_suite() {
  if [[ "${RUN_ADMIN}" != "true" ]]; then
    return
  fi
  if [[ -z "${ADMIN_BASE_URL}" || -z "${ADMIN_KEY}" ]]; then
    echo "admin_base_url/admin_key missing; skipping admin suite"
    return
  fi

  local provision_enabled
  provision_enabled="$(jq_bool_default '.admin_provision.enabled' 'false')"
  if [[ "${provision_enabled}" == "true" ]]; then
    echo "admin provision: applying providers/credentials/users..."
    local has_global_config
    has_global_config="$(jq -r '.admin_provision.global_config != null' "$CONFIG_PATH")"
    if [[ "${has_global_config}" == "true" ]]; then
      local global_config_payload
      global_config_payload="$(jq -c '.admin_provision.global_config' "$CONFIG_PATH")"
      admin_curl PUT "/admin/global_config" "${global_config_payload}"
    fi

    local provider_count
    provider_count="$(jq -r '.admin_provision.providers | length' "$CONFIG_PATH")"
    for ((i = 0; i < provider_count; i++)); do
      local name
      name="$(jq -r ".admin_provision.providers[$i].name" "$CONFIG_PATH")"
      local enabled
      enabled="$(jq_bool_default ".admin_provision.providers[$i].enabled" 'true')"
      local config_json
      config_json="$(jq -c ".admin_provision.providers[$i].config_json" "$CONFIG_PATH")"
      admin_curl PUT "/admin/providers/${name}" "{\"enabled\":${enabled},\"config_json\":${config_json}}"

      local cred_count
      cred_count="$(jq -r ".admin_provision.providers[$i].credentials | length" "$CONFIG_PATH")"
      for ((j = 0; j < cred_count; j++)); do
        local cred
        cred="$(jq -c ".admin_provision.providers[$i].credentials[$j]" "$CONFIG_PATH")"
        admin_curl POST "/admin/providers/${name}/credentials" "${cred}"
      done
    done

    local has_user
    has_user="$(jq -r '.admin_provision.user != null' "$CONFIG_PATH")"
    if [[ "${has_user}" == "true" ]]; then
      local user_id user_name user_enabled
      user_id="$(jq -r '.admin_provision.user.id' "$CONFIG_PATH")"
      user_name="$(jq -r '.admin_provision.user.name' "$CONFIG_PATH")"
      user_enabled="$(jq_bool_default '.admin_provision.user.enabled' 'true')"
      admin_curl PUT "/admin/users/${user_id}" "{\"name\":\"${user_name}\",\"enabled\":${user_enabled}}"

      local user_key
      user_key="$(jq -r '.admin_provision.user.key // empty' "$CONFIG_PATH")"
      local user_label
      user_label="$(jq -r '.admin_provision.user.label // empty' "$CONFIG_PATH")"
      local key_enabled
      key_enabled="$(jq_bool_default '.admin_provision.user.key_enabled' 'true')"
      if [[ -n "${user_key}" || -n "${user_label}" ]]; then
        admin_curl POST "/admin/users/${user_id}/keys" \
          "{\"key\":\"${user_key}\",\"label\":\"${user_label}\",\"enabled\":${key_enabled}}"
      fi
    fi
  fi

  local test_health test_global test_providers test_provider_details test_provider_creds test_credentials test_users test_user_keys
  test_health="$(jq_bool_default '.admin_tests.test_health' 'true')"
  test_global="$(jq_bool_default '.admin_tests.test_global_config' 'true')"
  test_providers="$(jq_bool_default '.admin_tests.test_providers' 'true')"
  test_provider_details="$(jq_bool_default '.admin_tests.test_provider_details' 'true')"
  test_provider_creds="$(jq_bool_default '.admin_tests.test_provider_credentials' 'true')"
  test_credentials="$(jq_bool_default '.admin_tests.test_credentials' 'true')"
  test_users="$(jq_bool_default '.admin_tests.test_users' 'true')"
  test_user_keys="$(jq_bool_default '.admin_tests.test_user_keys' 'false')"

  if [[ "${test_health}" == "true" ]]; then
    admin_curl GET "/admin/health"
  fi
  if [[ "${test_global}" == "true" ]]; then
    admin_curl GET "/admin/global_config"
  fi
  if [[ "${test_providers}" == "true" ]]; then
    admin_curl GET "/admin/providers"
  fi

  local provider_count
  provider_count="$(jq -r '.providers | length' "$CONFIG_PATH")"
  if [[ "${test_provider_details}" == "true" ]]; then
    for ((i = 0; i < provider_count; i++)); do
      local name
      name="$(jq -r ".providers[$i].name" "$CONFIG_PATH")"
      admin_curl GET "/admin/providers/${name}"
    done
  fi
  if [[ "${test_provider_creds}" == "true" ]]; then
    for ((i = 0; i < provider_count; i++)); do
      local name
      name="$(jq -r ".providers[$i].name" "$CONFIG_PATH")"
      admin_curl GET "/admin/providers/${name}/credentials"
    done
  fi
  if [[ "${test_credentials}" == "true" ]]; then
    admin_curl GET "/admin/credentials"
  fi
  if [[ "${test_users}" == "true" ]]; then
    local users_json
    users_json="$(admin_curl_json GET "/admin/users")"
    if [[ "${test_user_keys}" == "true" ]]; then
      local user_ids
      user_ids="$(echo "${users_json}" | jq -r '.users[].id')"
      for id in ${user_ids}; do
        admin_curl GET "/admin/users/${id}/keys"
      done
    fi
  fi
}

run_provider_suite() {
  local idx="$1"
  local name model anthropic_version gemini_version key_source
  name="$(jq -r ".providers[$idx].name" "$CONFIG_PATH")"
  model="$(jq -r ".providers[$idx].model" "$CONFIG_PATH")"
  anthropic_version="$(jq -r ".providers[$idx].anthropic_version // \"2023-06-01\"" "$CONFIG_PATH")"
  gemini_version="$(jq -r ".providers[$idx].gemini_version // \"v1beta\"" "$CONFIG_PATH")"
  key_source="$(jq -r ".providers[$idx].downstream_key_source // \"authorization_bearer\"" "$CONFIG_PATH")"

  local has_methods
  has_methods="$(jq -r ".providers[$idx].methods != null" "$CONFIG_PATH")"
  if [[ "${has_methods}" != "true" ]]; then
    curl_fail "provider '${name}' missing providers[$idx].methods (20 operation flags)"
  fi

  local requests=()
  local gemini_model="${model#models/}"

  method_enabled() {
    local method="$1"
    jq -r ".providers[$idx].methods.${method} // false" "$CONFIG_PATH"
  }

  add_gemini_model_op_requests() {
    local suffix="$1"
    local body="$2"
    local method="$3"
    if [[ "${gemini_version}" == "v1" || "${gemini_version}" == "both" ]]; then
      requests+=("${method}|/${name}/v1/models/${gemini_model}:${suffix}|${body}|")
    fi
    if [[ "${gemini_version}" == "v1beta" || "${gemini_version}" == "both" ]]; then
      requests+=("${method}|/${name}/v1beta/models/${gemini_model}:${suffix}|${body}|")
    fi
  }

  add_gemini_models_list_requests() {
    if [[ "${gemini_version}" == "v1" || "${gemini_version}" == "both" ]]; then
      requests+=("GET|/${name}/v1/models||")
    fi
    if [[ "${gemini_version}" == "v1beta" || "${gemini_version}" == "both" ]]; then
      requests+=("GET|/${name}/v1beta/models||")
    fi
  }

  add_gemini_models_get_requests() {
    if [[ "${gemini_version}" == "v1" || "${gemini_version}" == "both" ]]; then
      requests+=("GET|/${name}/v1/models/${gemini_model}||")
    fi
    if [[ "${gemini_version}" == "v1beta" || "${gemini_version}" == "both" ]]; then
      requests+=("GET|/${name}/v1beta/models/${gemini_model}||")
    fi
  }

  if [[ "$(method_enabled "claude_generate")" == "true" ]]; then
    requests+=("POST|/${name}/v1/messages|{\"model\":\"${model}\",\"max_tokens\":16,\"messages\":[{\"role\":\"user\",\"content\":\"ping\"}],\"stream\":false}|anthropic-version: ${anthropic_version}")
  fi

  if [[ "$(method_enabled "claude_generate_stream")" == "true" ]]; then
    requests+=("POST|/${name}/v1/messages|{\"model\":\"${model}\",\"max_tokens\":16,\"messages\":[{\"role\":\"user\",\"content\":\"ping\"}],\"stream\":true}|anthropic-version: ${anthropic_version}")
  fi

  if [[ "$(method_enabled "claude_count_tokens")" == "true" ]]; then
    requests+=("POST|/${name}/v1/messages/count_tokens|{\"model\":\"${model}\",\"messages\":[{\"role\":\"user\",\"content\":\"ping\"}]}|anthropic-version: ${anthropic_version}")
  fi

  if [[ "$(method_enabled "claude_models_list")" == "true" ]]; then
    requests+=("GET|/${name}/v1/models||")
  fi

  if [[ "$(method_enabled "claude_models_get")" == "true" ]]; then
    requests+=("GET|/${name}/v1/models/${model}||")
  fi

  local gemini_body="{\"contents\":[{\"role\":\"user\",\"parts\":[{\"text\":\"ping\"}]}]}"
  if [[ "$(method_enabled "gemini_generate")" == "true" ]]; then
    add_gemini_model_op_requests "generateContent" "${gemini_body}" "POST"
  fi

  if [[ "$(method_enabled "gemini_generate_stream")" == "true" ]]; then
    add_gemini_model_op_requests "streamGenerateContent" "${gemini_body}" "POST"
  fi

  if [[ "$(method_enabled "gemini_count_tokens")" == "true" ]]; then
    add_gemini_model_op_requests "countTokens" "${gemini_body}" "POST"
  fi

  if [[ "$(method_enabled "gemini_models_list")" == "true" ]]; then
    add_gemini_models_list_requests
  fi

  if [[ "$(method_enabled "gemini_models_get")" == "true" ]]; then
    add_gemini_models_get_requests
  fi

  if [[ "$(method_enabled "openai_chat_generate")" == "true" ]]; then
    requests+=("POST|/${name}/v1/chat/completions|{\"model\":\"${model}\",\"messages\":[{\"role\":\"user\",\"content\":\"ping\"}],\"stream\":false}|")
  fi

  if [[ "$(method_enabled "openai_chat_generate_stream")" == "true" ]]; then
    requests+=("POST|/${name}/v1/chat/completions|{\"model\":\"${model}\",\"messages\":[{\"role\":\"user\",\"content\":\"ping\"}],\"stream\":true}|")
  fi

  if [[ "$(method_enabled "openai_response_generate")" == "true" ]]; then
    requests+=("POST|/${name}/v1/responses|{\"model\":\"${model}\",\"input\":\"ping\",\"stream\":false}|")
  fi

  if [[ "$(method_enabled "openai_response_generate_stream")" == "true" ]]; then
    requests+=("POST|/${name}/v1/responses|{\"model\":\"${model}\",\"input\":\"ping\",\"stream\":true}|")
  fi

  if [[ "$(method_enabled "openai_input_tokens")" == "true" ]]; then
    requests+=("POST|/${name}/v1/responses/input_tokens|{\"model\":\"${model}\",\"input\":\"ping\"}|")
  fi

  if [[ "$(method_enabled "openai_models_list")" == "true" ]]; then
    requests+=("GET|/${name}/v1/models||")
  fi

  if [[ "$(method_enabled "openai_models_get")" == "true" ]]; then
    requests+=("GET|/${name}/v1/models/${model}||")
  fi

  if [[ "$(method_enabled "usage")" == "true" ]]; then
    requests+=("GET|/${name}/usage||")
  fi

  if [[ "$(method_enabled "oauth_start")" == "true" ]]; then
    requests+=("GET|/${name}/oauth|||redirect")
  fi

  if [[ "$(method_enabled "oauth_callback")" == "true" ]]; then
    requests+=("GET|/${name}/oauth/callback|||redirect")
  fi

  local i=0
  for req in "${requests[@]}"; do
    local method path body headers flag
    IFS='|' read -r method path body headers flag <<< "${req}"
    if [[ $i -gt 0 ]]; then
      sleep "${DELAY_SECS}"
    fi
    local allow_redirect="false"
    if [[ "${flag}" == "redirect" ]]; then
      allow_redirect="true"
    fi
    echo "[${name}] ${method} ${path}"
    proxy_curl "${method}" "${path}" "${key_source}" "${body}" "${headers}" "${allow_redirect}"
    i=$((i + 1))
  done
}

run_admin_suite

if [[ "${RUN_PROXY}" == "true" ]]; then
  provider_count="$(jq -r '.providers | length' "$CONFIG_PATH")"
  pids=()
  for ((i = 0; i < provider_count; i++)); do
    provider_enabled="$(jq_bool_default ".providers[$i].enabled" 'true')"
    if [[ "${provider_enabled}" != "true" ]]; then
      continue
    fi
    run_provider_suite "$i" &
    pids+=("$!")
  done
  failed=0
  for pid in "${pids[@]}"; do
    if ! wait "${pid}"; then
      failed=1
    fi
  done
  if [[ "${failed}" -ne 0 ]]; then
    exit 1
  fi
fi
