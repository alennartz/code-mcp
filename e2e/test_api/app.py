from typing import Any

from fastapi import Depends, FastAPI, HTTPException
from fastapi.openapi.utils import get_openapi

from test_api.auth import require_auth
from test_api.models import Owner, Pet, PetCreate, PetList, PetStatus, PetUpdate
from test_api.seed import seed_owners, seed_pets

app = FastAPI(
    title="Test API",
    version="1.0.0",
    description="E2E test API for code-mcp",
)


def _downgrade_schema(obj: Any) -> Any:
    """Recursively convert OpenAPI 3.1 nullable patterns to 3.0.3 style."""
    if isinstance(obj, dict):
        # Convert anyOf with null to nullable
        if "anyOf" in obj:
            non_null = [s for s in obj["anyOf"] if s != {"type": "null"}]
            if len(non_null) < len(obj["anyOf"]):
                # There was a null type — convert to nullable
                if len(non_null) == 1:
                    obj.update(non_null[0])
                    obj["nullable"] = True
                    del obj["anyOf"]
                else:
                    obj["anyOf"] = non_null
                    obj["nullable"] = True
        return {k: _downgrade_schema(v) for k, v in obj.items()}
    if isinstance(obj, list):
        return [_downgrade_schema(item) for item in obj]
    return obj


def custom_openapi() -> dict[str, Any]:
    if app.openapi_schema:
        return app.openapi_schema
    schema = get_openapi(
        title=app.title,
        version=app.version,
        description=app.description,
        routes=app.routes,
    )
    schema["openapi"] = "3.0.3"
    schema = _downgrade_schema(schema)
    app.openapi_schema = schema
    return schema


app.openapi = custom_openapi  # type: ignore[method-assign]

# In-memory state
db_pets: dict[int, Pet] = seed_pets()
db_owners: dict[int, Owner] = seed_owners()
next_pet_id: int = 5


@app.post("/reset", tags=["admin"])
def reset_data() -> dict[str, str]:
    global db_pets, db_owners, next_pet_id
    db_pets = seed_pets()
    db_owners = seed_owners()
    next_pet_id = 5
    return {"status": "ok"}


@app.get("/pets", tags=["pets"])
def list_pets(
    limit: int | None = None,
    status: PetStatus | None = None,
) -> PetList:
    pets = list(db_pets.values())
    if status is not None:
        pets = [p for p in pets if p.status == status]
    total = len(pets)
    if limit is not None:
        pets = pets[:limit]
    return PetList(items=pets, total=total)


@app.post("/pets", tags=["pets"], status_code=201, dependencies=[Depends(require_auth)])
def create_pet(body: PetCreate) -> Pet:
    global next_pet_id
    pet = Pet(id=next_pet_id, **body.model_dump())
    db_pets[next_pet_id] = pet
    next_pet_id += 1
    return pet


@app.get("/pets/{pet_id}", tags=["pets"])
def get_pet(pet_id: int) -> Pet:
    if pet_id not in db_pets:
        raise HTTPException(status_code=404, detail="Pet not found")
    return db_pets[pet_id]


@app.put("/pets/{pet_id}", tags=["pets"], dependencies=[Depends(require_auth)])
def update_pet(pet_id: int, body: PetUpdate) -> Pet:
    if pet_id not in db_pets:
        raise HTTPException(status_code=404, detail="Pet not found")
    existing = db_pets[pet_id]
    updated = existing.model_copy(update=body.model_dump(exclude_unset=True))
    db_pets[pet_id] = updated
    return updated


@app.delete("/pets/{pet_id}", tags=["pets"], dependencies=[Depends(require_auth)])
def delete_pet(pet_id: int) -> dict[str, str]:
    if pet_id not in db_pets:
        raise HTTPException(status_code=404, detail="Pet not found")
    del db_pets[pet_id]
    return {"status": "deleted"}


@app.get("/owners", tags=["owners"])
def list_owners() -> list[Owner]:
    return list(db_owners.values())


@app.get("/owners/{owner_id}/pets", tags=["owners"])
def list_owner_pets(owner_id: int) -> list[Pet]:
    if owner_id not in db_owners:
        raise HTTPException(status_code=404, detail="Owner not found")
    return [p for p in db_pets.values() if p.owner_id == owner_id]
