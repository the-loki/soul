#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include "flecs.h"

typedef struct soul_ecs_query_iter_t {
    ecs_iter_t iter;
    bool finished;
} soul_ecs_query_iter_t;

ecs_entity_t soul_ecs_component_init(
    ecs_world_t *world,
    const char *name,
    size_t size,
    size_t alignment
) {
    ecs_entity_desc_t entity_desc = {0};
    entity_desc.name = name;

    ecs_component_desc_t desc = {0};
    desc.entity = ecs_entity_init(world, &entity_desc);
    desc.type.size = size;
    desc.type.alignment = alignment;

    return ecs_component_init(world, &desc);
}

ecs_query_t *soul_ecs_query_init(
    ecs_world_t *world,
    const ecs_id_t *ids,
    const int16_t *inouts,
    int32_t count
) {
    if (count < 0 || count > FLECS_TERM_COUNT_MAX) {
        return NULL;
    }

    ecs_query_desc_t desc = {0};
    for (int32_t i = 0; i < count; i++) {
        desc.terms[i].id = ids[i];
        desc.terms[i].inout = inouts[i];
    }

    return ecs_query_init(world, &desc);
}

soul_ecs_query_iter_t *soul_ecs_query_iter(
    const ecs_world_t *world,
    const ecs_query_t *query
) {
    soul_ecs_query_iter_t *wrapper = calloc(1, sizeof(soul_ecs_query_iter_t));
    if (!wrapper) {
        return NULL;
    }

    wrapper->iter = ecs_query_iter(world, query);
    return wrapper;
}

bool soul_ecs_query_next(soul_ecs_query_iter_t *wrapper) {
    bool has_next = ecs_query_next(&wrapper->iter);
    wrapper->finished = !has_next;
    return has_next;
}

int32_t soul_ecs_query_iter_count(const soul_ecs_query_iter_t *wrapper) {
    return wrapper->iter.count;
}

void *soul_ecs_query_iter_field(
    const soul_ecs_query_iter_t *wrapper,
    size_t size,
    int8_t index
) {
    return ecs_field_w_size(&wrapper->iter, size, index);
}

void soul_ecs_query_iter_fini(soul_ecs_query_iter_t *wrapper) {
    if (!wrapper) {
        return;
    }
    if (!wrapper->finished) {
        ecs_iter_fini(&wrapper->iter);
    }
    free(wrapper);
}

ecs_entity_t soul_ecs_system_init(
    ecs_world_t *world,
    const ecs_id_t *ids,
    const int16_t *inouts,
    int32_t count,
    ecs_iter_action_t callback,
    void *ctx,
    ecs_ctx_free_t ctx_free
) {
    if (count < 0 || count > FLECS_TERM_COUNT_MAX) {
        return 0;
    }

    ecs_system_desc_t desc = {0};
    for (int32_t i = 0; i < count; i++) {
        desc.query.terms[i].id = ids[i];
        desc.query.terms[i].inout = inouts[i];
    }
    desc.callback = callback;
    desc.ctx = ctx;
    desc.ctx_free = ctx_free;
    desc.phase = EcsOnUpdate;

    return ecs_system_init(world, &desc);
}

int32_t soul_ecs_iter_count(const ecs_iter_t *iter) {
    return iter->count;
}

void *soul_ecs_iter_field(const ecs_iter_t *iter, size_t size, int8_t index) {
    return ecs_field_w_size(iter, size, index);
}

void *soul_ecs_iter_ctx(const ecs_iter_t *iter) {
    return iter->ctx;
}
