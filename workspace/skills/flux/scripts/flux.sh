#!/usr/bin/env bash
# Flux CLI helper — interact with Flux state engine

FLUX_URL="${FLUX_URL:-http://localhost:3000}"

# Helper function for API calls
api_call() {
    local method=$1
    local endpoint=$2
    local data=$3

    if [[ -n "$data" ]]; then
        curl -s -X "$method" "${FLUX_URL}${endpoint}" \
            -H "Content-Type: application/json" \
            -d "$data"
    else
        curl -s -X "$method" "${FLUX_URL}${endpoint}" \
            -H "Content-Type: application/json"
    fi
}

# Format JSON output (pretty print if jq available)
format_output() {
    if command -v jq &> /dev/null; then
        jq '.'
    else
        cat
    fi
}

case "${1:-}" in
    publish)
        stream="$2"
        source="$3"
        entity_id="$4"
        properties="$5"

        if [[ -z "$stream" || -z "$source" || -z "$entity_id" || -z "$properties" ]]; then
            echo "Usage: flux.sh publish STREAM SOURCE ENTITY_ID PROPERTIES_JSON"
            echo ""
            echo "Example:"
            echo "  flux.sh publish sensors agent-01 temp-sensor-01 '{\"temperature\":22.5}'"
            exit 1
        fi

        timestamp=$(date +%s)000

        event=$(cat <<EOF
{
  "stream": "${stream}",
  "source": "${source}",
  "timestamp": ${timestamp},
  "payload": {
    "entity_id": "${entity_id}",
    "properties": ${properties}
  }
}
EOF
)

        echo "Publishing event to Flux..."
        api_call POST "/api/events" "$event" | format_output
        ;;

    batch)
        events="$2"

        if [[ -z "$events" ]]; then
            echo "Usage: flux.sh batch EVENTS_JSON_ARRAY"
            echo ""
            echo "Example:"
            echo '  flux.sh batch '"'"'[{"stream":"sensors","source":"agent-01","payload":{"entity_id":"sensor-01","properties":{"temp":22}}}]'"'"''
            exit 1
        fi

        echo "Publishing batch to Flux..."
        api_call POST "/api/events/batch" "{\"events\": ${events}}" | format_output
        ;;

    get)
        entity_id="$2"

        if [[ -z "$entity_id" ]]; then
            echo "Usage: flux.sh get ENTITY_ID"
            exit 1
        fi

        api_call GET "/api/state/entities/${entity_id}" | format_output
        ;;

    list)
        prefix="$2"
        if [[ -n "$prefix" ]]; then
            api_call GET "/api/state/entities?prefix=${prefix}" | format_output
        else
            api_call GET "/api/state/entities" | format_output
        fi
        ;;

    delete)
        entity_id="$2"

        if [[ -z "$entity_id" ]]; then
            echo "Usage: flux.sh delete ENTITY_ID"
            echo "       flux.sh delete --prefix PREFIX"
            echo "       flux.sh delete --namespace NAMESPACE"
            exit 1
        fi

        if [[ "$entity_id" == "--prefix" ]]; then
            prefix="$3"
            echo "Batch deleting entities with prefix '${prefix}'..."
            api_call POST "/api/state/entities/delete" "{\"prefix\":\"${prefix}\"}" | format_output
        elif [[ "$entity_id" == "--namespace" ]]; then
            ns="$3"
            echo "Batch deleting entities in namespace '${ns}'..."
            api_call POST "/api/state/entities/delete" "{\"namespace\":\"${ns}\"}" | format_output
        else
            echo "Deleting entity '${entity_id}'..."
            api_call DELETE "/api/state/entities/${entity_id}" | format_output
        fi
        ;;

    health)
        echo "Testing Flux connection at ${FLUX_URL}..."
        response=$(api_call GET "/api/state/entities")

        if [[ $? -eq 0 && -n "$response" ]]; then
            echo "✓ Flux is reachable"
            entity_count=$(echo "$response" | grep -o '"id"' | wc -l)
            echo "  Entities in state: ${entity_count}"
        else
            echo "✗ Failed to reach Flux at ${FLUX_URL}"
            exit 1
        fi
        ;;

    *)
        echo "Flux CLI - Interact with Flux state engine"
        echo ""
        echo "Usage: flux.sh COMMAND [ARGS]"
        echo ""
        echo "Commands:"
        echo "  publish STREAM SOURCE ENTITY_ID PROPERTIES_JSON"
        echo "      Publish event to create/update entity"
        echo ""
        echo "  get ENTITY_ID"
        echo "      Query current state of entity"
        echo ""
        echo "  list [PREFIX]"
        echo "      List all entities (optionally filter by prefix)"
        echo ""
        echo "  delete ENTITY_ID"
        echo "  delete --prefix PREFIX"
        echo "  delete --namespace NAMESPACE"
        echo "      Delete entity or batch delete by filter"
        echo ""
        echo "  batch EVENTS_JSON_ARRAY"
        echo "      Publish multiple events at once"
        echo ""
        echo "  health"
        echo "      Test connection to Flux"
        echo ""
        echo "Examples:"
        echo "  flux.sh publish sensors agent-01 temp-01 '{\"temperature\":22.5}'"
        echo "  flux.sh get temp-01"
        echo "  flux.sh list"
        echo "  flux.sh list host-"
        echo "  flux.sh delete temp-01"
        echo "  flux.sh delete --prefix loadtest-"
        echo "  flux.sh health"
        echo ""
        echo "Environment:"
        echo "  FLUX_URL=${FLUX_URL}"
        exit 1
        ;;
esac
