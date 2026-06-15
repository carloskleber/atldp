import plotly.graph_objs as go
import pandas as pd
import requests

# Function to fetch elevation data from OpenElevation (as before)
def fetch_elevation_data(lat, lon, num_points=100):
    coords = [(lat + 0.01 * (i - num_points / 2), lon + 0.01 * (j - num_points / 2)) for i in range(num_points) for j in range(num_points)]
    response = requests.post("https://api.open-elevation.com/api/v1/lookup", json={"locations": [{"latitude": c[0], "longitude": c[1]} for c in coords]})
    data = response.json()
    return pd.DataFrame([(d["latitude"], d["longitude"], d["elevation"]) for d in data["results"]], columns=["latitude", "longitude", "elevation"])

# Fetch elevation data
lat, lon = 37.76, -122.44
num_points = 100
data = fetch_elevation_data(lat, lon, num_points)

# Reshape data for surface plot
data_pivot = data.pivot(index='latitude', columns='longitude', values='elevation')

# Create the surface plot
fig = go.Figure(data=[go.Surface(z=data_pivot.values, x=data_pivot.columns, y=data_pivot.index)])
fig.update_layout(
    title='3D Elevation Map',
    scene=dict(
        xaxis_title='Longitude',
        yaxis_title='Latitude',
        zaxis_title='Elevation',
        aspectratio=dict(x=1, y=1, z=0.5),
    ),
    autosize=False,
    width=800,
    height=800,
)

# Show the plot
fig.show()
