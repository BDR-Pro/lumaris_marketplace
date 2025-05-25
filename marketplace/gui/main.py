#!/usr/bin/env python3
"""
Lumaris Marketplace GUI
A simple GUI for the Lumaris Marketplace that allows users to choose between buyer and seller roles.
"""

import sys
import json
import asyncio
import websockets
import tkinter as tk
from tkinter import ttk, scrolledtext, messagebox
from datetime import datetime
import threading
from rich.console import Console
from rich.panel import Panel
from rich.text import Text
from rich import box

# Initialize Rich console
console = Console()

# WebSocket connection
ws_connection = None
user_id = None
user_role = None

# Create a function to connect to the WebSocket server
async def connect_to_server(server_url, role, user_id):
    try:
        connection = await websockets.connect(server_url)
        console.print(f"[green]Connected to {server_url} as {role} (ID: {user_id})[/green]")
        return connection
    except Exception as e:
        console.print(f"[red]Error connecting to server: {e}[/red]")
        return None

# Function to send a message to the server
async def send_message(connection, message):
    if connection:
        try:
            await connection.send(json.dumps(message))
            console.print(f"[blue]Sent: {message}[/blue]")
        except Exception as e:
            console.print(f"[red]Error sending message: {e}[/red]")

# Function to receive messages from the server
async def receive_messages(connection, output_text):
    if connection:
        try:
            while True:
                message = await connection.recv()
                console.print(f"[green]Received: {message}[/green]")
                
                # Parse the message
                try:
                    data = json.loads(message)
                    
                    # Handle different message types
                    if "type" in data:
                        if data["type"] == "buyer_stats" and "stats" in data and "formatted_stats" in data["stats"]:
                            # Display buyer stats
                            output_text.delete(1.0, tk.END)
                            output_text.insert(tk.END, data["stats"]["formatted_stats"])
                        elif data["type"] == "seller_stats" and "stats" in data and "formatted_stats" in data["stats"]:
                            # Display seller stats
                            output_text.delete(1.0, tk.END)
                            output_text.insert(tk.END, data["stats"]["formatted_stats"])
                        else:
                            # Display other messages
                            output_text.insert(tk.END, f"\n{json.dumps(data, indent=2)}\n")
                    else:
                        # Display raw message
                        output_text.insert(tk.END, f"\n{message}\n")
                except json.JSONDecodeError:
                    # Display raw message if not JSON
                    output_text.insert(tk.END, f"\n{message}\n")
                
                # Auto-scroll to the bottom
                output_text.see(tk.END)
        except websockets.exceptions.ConnectionClosed:
            console.print("[yellow]Connection closed[/yellow]")
            output_text.insert(tk.END, "\n--- Connection closed ---\n")
        except Exception as e:
            console.print(f"[red]Error receiving messages: {e}[/red]")
            output_text.insert(tk.END, f"\n--- Error: {e} ---\n")

# Function to start the WebSocket connection in a separate thread
def start_websocket_thread(server_url, role, user_id, output_text):
    async def run_websocket():
        global ws_connection
        ws_connection = await connect_to_server(server_url, role, user_id)
        if ws_connection:
            # Start receiving messages
            await receive_messages(ws_connection, output_text)
    
    # Create and start the thread
    threading.Thread(target=lambda: asyncio.run(run_websocket()), daemon=True).start()

# Function to get statistics based on role
def get_statistics(role, id_value, output_text):
    async def send_stats_request():
        if not ws_connection:
            messagebox.showerror("Error", "Not connected to server")
            return
        
        if role == "buyer":
            # Request buyer statistics
            await send_message(ws_connection, {
                "type": "get_buyer_stats",
                "buyer_id": id_value
            })
        else:
            # Request seller statistics
            await send_message(ws_connection, {
                "type": "get_seller_stats",
                "seller_id": id_value
            })
    
    # Run the async function
    asyncio.run(send_stats_request())

# Function to create a VM (for buyers)
def create_vm(buyer_id, job_id, vcpu_count, mem_size, output_text):
    async def send_create_vm_request():
        if not ws_connection:
            messagebox.showerror("Error", "Not connected to server")
            return
        
        # Request VM creation
        await send_message(ws_connection, {
            "type": "create_vm",
            "buyer_id": buyer_id,
            "job_id": int(job_id),
            "vcpu_count": int(vcpu_count),
            "mem_size_mib": int(mem_size)
        })
    
    # Run the async function
    asyncio.run(send_create_vm_request())

# Function to register a node (for sellers)
def register_node(seller_id, cpu_cores, memory_mb, output_text):
    async def send_register_node_request():
        if not ws_connection:
            messagebox.showerror("Error", "Not connected to server")
            return
        
        # Request node registration
        await send_message(ws_connection, {
            "type": "node_registration",
            "seller_id": seller_id,
            "capabilities": {
                "cpu_cores": float(cpu_cores),
                "memory_mb": int(memory_mb)
            }
        })
    
    # Run the async function
    asyncio.run(send_register_node_request())

# Function to create the buyer interface
def create_buyer_interface(parent, user_id):
    # Clear the parent frame
    for widget in parent.winfo_children():
        widget.destroy()
    
    # Create the buyer interface
    ttk.Label(parent, text=f"Buyer Interface - ID: {user_id}", font=("Arial", 16)).pack(pady=10)
    
    # Create a frame for VM creation
    vm_frame = ttk.LabelFrame(parent, text="Create VM")
    vm_frame.pack(fill="x", padx=10, pady=5)
    
    # Job ID
    ttk.Label(vm_frame, text="Job ID:").grid(row=0, column=0, padx=5, pady=5, sticky="w")
    job_id_entry = ttk.Entry(vm_frame)
    job_id_entry.grid(row=0, column=1, padx=5, pady=5, sticky="ew")
    job_id_entry.insert(0, "123")
    
    # vCPU count
    ttk.Label(vm_frame, text="vCPU Count:").grid(row=1, column=0, padx=5, pady=5, sticky="w")
    vcpu_entry = ttk.Entry(vm_frame)
    vcpu_entry.grid(row=1, column=1, padx=5, pady=5, sticky="ew")
    vcpu_entry.insert(0, "2")
    
    # Memory size
    ttk.Label(vm_frame, text="Memory (MiB):").grid(row=2, column=0, padx=5, pady=5, sticky="w")
    mem_entry = ttk.Entry(vm_frame)
    mem_entry.grid(row=2, column=1, padx=5, pady=5, sticky="ew")
    mem_entry.insert(0, "1024")
    
    # Create VM button
    create_vm_button = ttk.Button(
        vm_frame, 
        text="Create VM", 
        command=lambda: create_vm(user_id, job_id_entry.get(), vcpu_entry.get(), mem_entry.get(), output_text)
    )
    create_vm_button.grid(row=3, column=0, columnspan=2, padx=5, pady=10)
    
    # Create a frame for statistics
    stats_frame = ttk.LabelFrame(parent, text="Statistics")
    stats_frame.pack(fill="x", padx=10, pady=5)
    
    # Get statistics button
    get_stats_button = ttk.Button(
        stats_frame, 
        text="Get Buyer Statistics", 
        command=lambda: get_statistics("buyer", user_id, output_text)
    )
    get_stats_button.pack(padx=5, pady=10)
    
    # Create output text area
    output_text = scrolledtext.ScrolledText(parent, height=20)
    output_text.pack(fill="both", expand=True, padx=10, pady=10)
    
    # Configure grid weights
    vm_frame.columnconfigure(1, weight=1)
    
    return output_text

# Function to create the seller interface
def create_seller_interface(parent, user_id):
    # Clear the parent frame
    for widget in parent.winfo_children():
        widget.destroy()
    
    # Create the seller interface
    ttk.Label(parent, text=f"Seller Interface - ID: {user_id}", font=("Arial", 16)).pack(pady=10)
    
    # Create a frame for node registration
    node_frame = ttk.LabelFrame(parent, text="Register Node")
    node_frame.pack(fill="x", padx=10, pady=5)
    
    # CPU cores
    ttk.Label(node_frame, text="CPU Cores:").grid(row=0, column=0, padx=5, pady=5, sticky="w")
    cpu_entry = ttk.Entry(node_frame)
    cpu_entry.grid(row=0, column=1, padx=5, pady=5, sticky="ew")
    cpu_entry.insert(0, "4")
    
    # Memory
    ttk.Label(node_frame, text="Memory (MB):").grid(row=1, column=0, padx=5, pady=5, sticky="w")
    mem_entry = ttk.Entry(node_frame)
    mem_entry.grid(row=1, column=1, padx=5, pady=5, sticky="ew")
    mem_entry.insert(0, "8192")
    
    # Register node button
    register_button = ttk.Button(
        node_frame, 
        text="Register Node", 
        command=lambda: register_node(user_id, cpu_entry.get(), mem_entry.get(), output_text)
    )
    register_button.grid(row=2, column=0, columnspan=2, padx=5, pady=10)
    
    # Create a frame for statistics
    stats_frame = ttk.LabelFrame(parent, text="Statistics")
    stats_frame.pack(fill="x", padx=10, pady=5)
    
    # Get statistics button
    get_stats_button = ttk.Button(
        stats_frame, 
        text="Get Seller Statistics", 
        command=lambda: get_statistics("seller", user_id, output_text)
    )
    get_stats_button.pack(padx=5, pady=10)
    
    # Create output text area
    output_text = scrolledtext.ScrolledText(parent, height=20)
    output_text.pack(fill="both", expand=True, padx=10, pady=10)
    
    # Configure grid weights
    node_frame.columnconfigure(1, weight=1)
    
    return output_text

# Function to handle role selection
def select_role(role, id_entry, server_entry, main_frame, root):
    global user_id, user_role
    
    # Get user ID and server URL
    user_id = id_entry.get()
    server_url = server_entry.get()
    user_role = role
    
    if not user_id:
        messagebox.showerror("Error", "Please enter a user ID")
        return
    
    if not server_url:
        messagebox.showerror("Error", "Please enter a server URL")
        return
    
    # Create the appropriate interface based on role
    if role == "buyer":
        output_text = create_buyer_interface(main_frame, user_id)
    else:
        output_text = create_seller_interface(main_frame, user_id)
    
    # Start WebSocket connection
    start_websocket_thread(server_url, role, user_id, output_text)
    
    # Update window title
    root.title(f"Lumaris Marketplace - {role.capitalize()} Mode")

# Main function to create the GUI
def main():
    # Create the main window
    root = tk.Tk()
    root.title("Lumaris Marketplace")
    root.geometry("800x600")
    
    # Create a style
    style = ttk.Style()
    style.configure("TButton", padding=6, relief="flat", background="#ccc")
    
    # Create a frame for the role selection
    role_frame = ttk.LabelFrame(root, text="Select Role")
    role_frame.pack(fill="x", padx=10, pady=10)
    
    # User ID
    ttk.Label(role_frame, text="User ID:").grid(row=0, column=0, padx=5, pady=5, sticky="w")
    id_entry = ttk.Entry(role_frame)
    id_entry.grid(row=0, column=1, padx=5, pady=5, sticky="ew")
    id_entry.insert(0, f"user-{datetime.now().strftime('%Y%m%d%H%M%S')}")
    
    # Server URL
    ttk.Label(role_frame, text="Server URL:").grid(row=1, column=0, padx=5, pady=5, sticky="w")
    server_entry = ttk.Entry(role_frame)
    server_entry.grid(row=1, column=1, padx=5, pady=5, sticky="ew")
    server_entry.insert(0, "ws://localhost:3030/ws")
    
    # Role buttons
    buyer_button = ttk.Button(
        role_frame, 
        text="Enter as Buyer", 
        command=lambda: select_role("buyer", id_entry, server_entry, main_frame, root)
    )
    buyer_button.grid(row=2, column=0, padx=5, pady=10)
    
    seller_button = ttk.Button(
        role_frame, 
        text="Enter as Seller", 
        command=lambda: select_role("seller", id_entry, server_entry, main_frame, root)
    )
    seller_button.grid(row=2, column=1, padx=5, pady=10)
    
    # Create a main frame for the interface
    main_frame = ttk.Frame(root)
    main_frame.pack(fill="both", expand=True, padx=10, pady=10)
    
    # Configure grid weights
    role_frame.columnconfigure(1, weight=1)
    
    # Start the main loop
    root.mainloop()

# Clean up WebSocket connection on exit
def cleanup():
    if ws_connection:
        asyncio.run(ws_connection.close())

if __name__ == "__main__":
    try:
        # Print banner
        console.print(Panel(
            Text("Lumaris Marketplace GUI", justify="center", style="bold blue"),
            box=box.DOUBLE,
            expand=False
        ))
        
        # Start the GUI
        main()
    except KeyboardInterrupt:
        console.print("[yellow]Exiting...[/yellow]")
    finally:
        cleanup()

